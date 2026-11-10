package com.wusla.hams.ime

import android.view.HapticFeedbackConstants
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.wusla.hams.core.Dictation
import com.wusla.hams.core.DictationKind
import com.wusla.hams.core.FieldKind
import com.wusla.hams.core.Hams
import com.wusla.hams.core.Pad
import com.wusla.hams.core.Phase
import com.wusla.hams.core.io
import com.wusla.hams.R
import com.wusla.hams.ui.components.HamsIcon
import com.wusla.hams.ui.components.Mark
import com.wusla.hams.ui.components.Orb
import com.wusla.hams.ui.components.OrbMood
import com.wusla.hams.ui.screens.orbMood
import com.wusla.hams.ui.theme.HamsShape
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** Rewrite chips shown after a dictation: (style id, label). */
private val REWRITES = listOf("messaging" to "Casual", "email" to "Formal", "docs" to "Tidy")

/**
 * The keyboard: one fixed height in every phase so the app above never jumps
 * (the same rule as the desktop overlay's fixed pill).
 *
 *   ┌ strip: Heard / Written of the last dictation, Undo + rewrite chips ┐
 *   │ mic pad: hold to talk · double-tap for hands-free · slide to cancel │
 *   └ 🌐  ,  .  ?  [ space ]  ⌫  ⏎                                       ┘
 */
@Composable
fun KeyboardScreen(service: HamsKeyboardService) {
    val c = Tokens.colors
    val editor by service.editor.collectAsState()
    val status by Hams.status.collectAsState()
    val level by Hams.level.collectAsState()
    val partial by Hams.partial.collectAsState()
    val dataVersion by Hams.dataVersion.collectAsState()
    val ready by Hams.ready.collectAsState()
    val scope = rememberCoroutineScope()

    var last by remember { mutableStateOf<Dictation?>(null) }
    var canUndo by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf<String?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }

    // After each dictation (history is written right after delivery), refresh the strip.
    LaunchedEffect(dataVersion, status.phase, ready) {
        val engine = Hams.engineOrNull ?: return@LaunchedEffect
        io { lastDictation() to canUndo() }.onSuccess { (dictation, undoable) ->
            last = dictation
            canUndo = undoable
        }
        if (engine.isRecording()) notice = null
    }
    LaunchedEffect(status) {
        if (status.phase == Phase.ERROR) {
            notice = status.message
            delay(5_000)
            if (notice == status.message) notice = null
        }
    }

    val recording = status.phase == Phase.RECORDING
    Column(
        Modifier
            .fillMaxWidth()
            .background(c.ground)
            .navigationBarsPadding()
            .height(272.dp)
            .padding(horizontal = 8.dp, vertical = 6.dp),
    ) {
        Strip(
            modifier = Modifier.fillMaxWidth().height(70.dp),
            last = last.takeIf { canUndo },
            busy = busy,
            notice = notice,
            recording = recording,
            startedAt = status.startedAt,
            locked = status.locked,
            kind = status.kind,
            partial = partial,
            onUndo = {
                scope.launch {
                    io { undoLast() }.onSuccess { if (it) canUndo = false }
                }
            },
            onRewrite = { style, label ->
                busy = label
                scope.launch {
                    io { restyleLast(style) }.onFailure { notice = it.message }
                    busy = null
                }
            },
        )
        Spacer(Modifier.height(6.dp))
        MicPad(
            modifier = Modifier.fillMaxWidth().weight(1f),
            editor = editor,
            ready = ready,
            phase = status.phase,
            locked = status.locked,
            level = level,
            onOpenApp = { service.openApp(it) },
        )
        Spacer(Modifier.height(6.dp))
        KeyRow(service, editor)
    }
}

// ---- the strip above the pad ------------------------------------------------------------

@Composable
private fun Strip(
    modifier: Modifier,
    last: Dictation?,
    busy: String?,
    notice: String?,
    recording: Boolean,
    startedAt: Long,
    locked: Boolean,
    kind: DictationKind,
    partial: String,
    onUndo: () -> Unit,
    onRewrite: (String, String) -> Unit,
) {
    val c = Tokens.colors
    Column(modifier.padding(horizontal = 6.dp), verticalArrangement = Arrangement.SpaceBetween) {
        when {
            recording -> Listening(startedAt, locked, kind, partial, Modifier.weight(1f))
            notice != null -> Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                Box(Modifier.size(8.dp).clip(RoundedCornerShape(50)).background(c.live))
                Spacer(Modifier.width(8.dp))
                Text(notice, style = HamsType.body, color = c.text, maxLines = 3, overflow = TextOverflow.Ellipsis)
            }
            last != null -> Box(Modifier.weight(1f), contentAlignment = Alignment.CenterStart) {
                Text(
                    last.text, style = HamsType.body.copy(fontSize = 15.sp), color = c.text,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
            else -> Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                Mark(size = 16, tint = c.voice)
                Spacer(Modifier.width(8.dp))
                Text(
                    "Hold the orb and speak. Hams cleans it up.",
                    style = HamsType.small, color = c.muted, maxLines = 2,
                )
            }
        }
        if (last != null && notice == null && !recording) {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                StripChip("Undo", icon = R.drawable.ic_undo, enabled = busy == null, onClick = onUndo)
                REWRITES.forEach { (style, label) ->
                    StripChip(if (busy == label) "$label…" else label, enabled = busy == null) { onRewrite(style, label) }
                }
            }
        }
    }
}

/** While recording, the strip shows the time and the words as they come. */
@Composable
private fun Listening(startedAt: Long, locked: Boolean, kind: DictationKind, partial: String, modifier: Modifier) {
    val c = Tokens.colors
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(startedAt) {
        while (isActive) {
            now = System.currentTimeMillis()
            delay(250)
        }
    }
    val pulse = rememberInfiniteTransition(label = "dot")
    val alpha by pulse.animateFloat(0.35f, 1f, infiniteRepeatable(tween(600), RepeatMode.Reverse), label = "alpha")
    val seconds = if (startedAt > 0) ((now - startedAt) / 1000).coerceAtLeast(0) else 0
    Column(modifier, verticalArrangement = Arrangement.Center) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(8.dp).clip(RoundedCornerShape(50)).background(c.voice.copy(alpha = alpha)))
            Spacer(Modifier.width(8.dp))
            Text(
                "%d:%02d".format(seconds / 60, seconds % 60) + when {
                    kind == DictationKind.COMMAND -> "  ·  say the change"
                    locked -> "  ·  tap the orb to finish"
                    else -> "  ·  let go to insert, slide to cancel"
                },
                style = HamsType.label, color = c.text,
            )
        }
        if (partial.isNotBlank()) {
            Spacer(Modifier.height(4.dp))
            Text(partial, style = HamsType.small, color = c.muted, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
    }
}

@Composable
private fun StripChip(text: String, icon: Int? = null, enabled: Boolean = true, onClick: () -> Unit) {
    val c = Tokens.colors
    Row(
        Modifier
            .clip(RoundedCornerShape(50))
            .background(c.surface)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 14.dp, vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            HamsIcon(icon, tint = if (enabled) c.text else c.faint, size = 15.dp)
            Spacer(Modifier.width(5.dp))
        }
        Text(text, style = HamsType.label, color = if (enabled) c.text else c.faint)
    }
}

// ---- the mic pad ----------------------------------------------------------------------

@Composable
private fun MicPad(
    modifier: Modifier,
    editor: HamsKeyboardService.Editor,
    ready: Boolean,
    phase: Phase,
    locked: Boolean,
    level: Float,
    onOpenApp: (String?) -> Unit,
) {
    val c = Tokens.colors
    val view = LocalView.current
    val engine = Hams.engineOrNull
    val recording = phase == Phase.RECORDING
    val working = phase == Phase.TRANSCRIBING || phase == Phase.POLISHING || phase == Phase.DELIVERING
    val pad = if (editor.hasSelection) Pad.COMMAND else Pad.DICTATE

    // Setup problems the keyboard can't fix itself: send the user to the app.
    val blocker: Pair<String, String?>? = when {
        !ready -> "Starting…" to "-"
        engine == null -> "Hams couldn't start. Open the app to see why." to null
        editor.kind == FieldKind.PASSWORD -> "Hams is off in password fields." to "-"
        !editor.micAllowed -> "Tap to allow the microphone in Hams." to "permission"
        !editor.modelReady -> "Tap to download the speech model." to "model"
        else -> null
    }

    val background by animateColorAsState(if (recording) c.voiceSoft else c.surface, label = "pad")
    val density = LocalDensity.current
    val cancelDistance = with(density) { 96.dp.toPx() }
    var cancelled by remember { mutableStateOf(false) }

    Box(
        modifier
            .clip(RoundedCornerShape(20.dp))
            .background(background)
            .semantics { contentDescription = if (recording) "Recording. Release to insert." else "Hold to talk" }
            .then(
                if (blocker != null) {
                    Modifier.clickable {
                        when (val target = blocker.second) {
                            "-" -> Unit
                            else -> onOpenApp(target)
                        }
                    }
                } else {
                    Modifier.pointerInput(pad) {
                        awaitEachGesture {
                            val down = awaitFirstDown(requireUnconsumed = false)
                            cancelled = false
                            view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                            engine?.press(pad)
                            // Hold until every finger is up; sliding sideways cancels.
                            while (true) {
                                val event = awaitPointerEvent()
                                val change = event.changes.firstOrNull { it.id == down.id }
                                if (change != null && !cancelled && kotlin.math.abs(change.position.x - down.position.x) > cancelDistance) {
                                    cancelled = true
                                    view.performHapticFeedback(HapticFeedbackConstants.REJECT)
                                    engine?.cancel()
                                }
                                if (event.changes.none { it.pressed }) break
                            }
                            if (!cancelled) view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_RELEASE)
                            engine?.release(pad)
                        }
                    }
                },
            ),
        contentAlignment = Alignment.Center,
    ) {
        val mood = when {
            !ready -> OrbMood.Working
            blocker != null -> OrbMood.Off
            else -> orbMood(phase, ready = true)
        }
        Row(Modifier.fillMaxSize().padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            PadSide(
                when {
                    blocker != null -> blocker.first
                    recording && locked -> "Hands-free"
                    recording -> "Listening"
                    working -> when (phase) {
                        Phase.POLISHING -> "Polishing…"
                        Phase.DELIVERING -> "Writing…"
                        else -> "Transcribing…"
                    }
                    pad == Pad.COMMAND -> "Say how to change it"
                    else -> "Hold to talk"
                },
                Modifier.weight(1f), TextAlign.End, strong = true,
            )
            Orb(mood, level, diameter = 112.dp, command = pad == Pad.COMMAND)
            PadSide(
                when {
                    blocker != null -> ""
                    recording && locked -> "Tap to finish"
                    recording -> "Let go to insert"
                    working -> ""
                    pad == Pad.COMMAND -> "\"friendlier\", \"in Malayalam\""
                    else -> "Double-tap for hands-free"
                },
                Modifier.weight(1f), TextAlign.Start, strong = false,
            )
        }
    }
}

@Composable
private fun PadSide(text: String, modifier: Modifier, align: TextAlign, strong: Boolean) {
    val c = Tokens.colors
    Text(
        text, modifier = modifier.padding(horizontal = 6.dp),
        style = if (strong) HamsType.bodyStrong else HamsType.small,
        color = if (strong) c.text else c.muted, textAlign = align, maxLines = 3, overflow = TextOverflow.Ellipsis,
    )
}

// ---- the essential keys ------------------------------------------------------------------

@Composable
private fun KeyRow(service: HamsKeyboardService, editor: HamsKeyboardService.Editor) {
    val enterLabel = when (editor.enter) {
        FieldKinds.Enter.GO -> "Go"
        FieldKinds.Enter.SEARCH -> "Search"
        FieldKinds.Enter.SEND -> "Send"
        FieldKinds.Enter.NEXT -> "Next"
        FieldKinds.Enter.DONE -> "Done"
        FieldKinds.Enter.PREVIOUS -> "Back"
        FieldKinds.Enter.NEWLINE -> "⏎"
    }
    Row(Modifier.fillMaxWidth().height(50.dp), horizontalArrangement = Arrangement.spacedBy(5.dp)) {
        Key("", Modifier.weight(1f), icon = R.drawable.ic_globe, description = "Switch keyboard", onLongPress = { service.showPicker() }) { service.switchKeyboard() }
        Key(",", Modifier.weight(0.8f)) { service.type(",") }
        Key(".", Modifier.weight(0.8f)) { service.type(".") }
        Key("?", Modifier.weight(0.8f)) { service.type("?") }
        Key("space", Modifier.weight(3f), quiet = true) { service.type(" ") }
        RepeatKey(Modifier.weight(1.2f), description = "Delete", onLongPress = { service.deleteWord() }) { service.backspace() }
        Key(
            if (editor.enter == FieldKinds.Enter.NEWLINE) "" else enterLabel, Modifier.weight(1.4f),
            primary = true, icon = if (editor.enter == FieldKinds.Enter.NEWLINE) R.drawable.ic_enter else null,
            description = if (editor.enter == FieldKinds.Enter.NEWLINE) "Enter" else enterLabel,
        ) { service.enter() }
    }
}

@Composable
private fun RowScope.Key(
    label: String,
    modifier: Modifier,
    primary: Boolean = false,
    quiet: Boolean = false,
    icon: Int? = null,
    description: String? = null,
    onLongPress: (() -> Unit)? = null,
    onTap: () -> Unit,
) {
    val c = Tokens.colors
    val view = LocalView.current
    Box(
        modifier
            .fillMaxHeight()
            .clip(RoundedCornerShape(HamsShape.key))
            .background(if (primary) c.primary else c.surface)
            .semantics { contentDescription = description ?: label }
            .pointerInput(onLongPress) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = false)
                    view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                    var longPressed = false
                    if (onLongPress != null) {
                        val released = withTimeoutOrNull(450) { waitUntilUp() }
                        if (released == null) {
                            longPressed = true
                            view.performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
                            onLongPress()
                            waitUntilUp()
                        }
                    } else {
                        waitUntilUp()
                    }
                    if (!longPressed) onTap()
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        val ink = when {
            primary -> c.primaryInk
            quiet -> c.muted
            else -> c.text
        }
        if (icon != null) {
            HamsIcon(icon, tint = ink, size = 22.dp)
        } else {
            Text(label, style = if (label.length > 2) HamsType.label else HamsType.heading, color = ink)
        }
    }
}

/** Wait until every finger is up. */
private suspend fun AwaitPointerEventScope.waitUntilUp() {
    while (true) {
        if (awaitPointerEvent().changes.none { it.pressed }) return
    }
}

/** Backspace: repeats while held, speeding up; after a second and a half it deletes whole words. */
@Composable
private fun RowScope.RepeatKey(modifier: Modifier, description: String, onLongPress: () -> Unit, onTap: () -> Unit) {
    val c = Tokens.colors
    val view = LocalView.current
    val scope = rememberCoroutineScope()
    Box(
        modifier
            .fillMaxHeight()
            .clip(RoundedCornerShape(HamsShape.key))
            .background(c.surface)
            .semantics { contentDescription = description }
            .pointerInput(Unit) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = false)
                    view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                    onTap()
                    val repeater = scope.launch {
                        delay(450)
                        val started = System.currentTimeMillis()
                        var interval = 110L
                        while (isActive) {
                            if (System.currentTimeMillis() - started > 1_500) onLongPress() else onTap()
                            delay(interval)
                            interval = (interval * 0.9).toLong().coerceAtLeast(45)
                        }
                    }
                    waitUntilUp()
                    repeater.cancel()
                }
            },
        contentAlignment = Alignment.Center,
    ) { HamsIcon(R.drawable.ic_backspace, tint = c.text, size = 22.dp) }
}
