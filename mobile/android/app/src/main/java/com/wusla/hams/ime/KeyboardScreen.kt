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
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.foundation.shape.CircleShape
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
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.wusla.hams.core.Dictation
import com.wusla.hams.core.DictationKind
import com.wusla.hams.core.FieldKind
import com.wusla.hams.core.Hams
import com.wusla.hams.core.Lang
import com.wusla.hams.core.Mic
import com.wusla.hams.core.Parse
import com.wusla.hams.core.Pad
import com.wusla.hams.core.Phase
import com.wusla.hams.core.SettingsDoc
import com.wusla.hams.core.io
import com.wusla.hams.R
import com.wusla.hams.ui.components.HamsIcon
import com.wusla.hams.ui.components.Mark
import com.wusla.hams.ui.components.DotMode
import com.wusla.hams.ui.components.DotWave
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
 *   │ dot pad: hold to talk · double-tap for hands-free · slide to cancel │
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
    // Dictation language ("auto" or a code) and the user's languages, for the chip.
    var language by remember { mutableStateOf("auto") }
    var languages by remember { mutableStateOf<List<String>>(emptyList()) }
    // A bigger speech pack would suit the user's languages better, and they haven't dismissed the hint.
    var picker by remember { mutableStateOf(false) }
    var catalog by remember { mutableStateOf<Map<String, Lang>>(emptyMap()) }
    var packTip by remember { mutableStateOf(false) }
    val tipPrefs = remember { service.getSharedPreferences("hams_keyboard", android.content.Context.MODE_PRIVATE) }

    // After each dictation (history is written right after delivery), refresh the strip.
    LaunchedEffect(dataVersion, status.phase, ready) {
        val engine = Hams.engineOrNull ?: return@LaunchedEffect
        io { lastDictation() to canUndo() }.onSuccess { (dictation, undoable) ->
            last = dictation
            canUndo = undoable
        }
        io { SettingsDoc.parse(settingsJson()) }.onSuccess {
            language = it.language
            languages = it.languages
            val suggested = engine.recommendedModel()
            packTip = suggested.startsWith("small") && !it.model.startsWith("small") &&
                !tipPrefs.getBoolean("pack_tip_dismissed", false)
        }
        if (catalog.isEmpty()) io { Parse.languages(languagesJson()) }.onSuccess { list -> catalog = list.associateBy { it.code } }
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
    val paused by Mic.paused.collectAsState()
    val seconds = elapsedSeconds(status.startedAt, recording)
    Box(Modifier.fillMaxWidth().background(c.ground).navigationBarsPadding().height(272.dp)) {
    Column(Modifier.fillMaxSize().padding(horizontal = 8.dp, vertical = 6.dp)) {
        Strip(
            modifier = Modifier.fillMaxWidth().height(70.dp),
            last = last.takeIf { canUndo },
            busy = busy,
            notice = notice,
            recording = recording,
            paused = paused,
            seconds = seconds,
            kind = status.kind,
            partial = partial,
            language = language,
            showLanguage = ready,
            languages = languages,
            onLanguage = { service.openApp("languages") },
            onLongLanguage = { picker = true },
            onPickLanguage = { code ->
                language = code
                scope.launch { io { setLanguage(code) }.onFailure { notice = it.message } }
            },
            packTip = packTip,
            onPackTip = { service.openApp("languages") },
            onDismissPackTip = {
                packTip = false
                tipPrefs.edit().putBoolean("pack_tip_dismissed", true).apply()
            },
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
            paused = paused,
            onNotice = { notice = it },
            onOpenApp = { service.openApp(it) },
        )
        Spacer(Modifier.height(6.dp))
        KeyRow(service, editor)
    }
    if (picker) {
        LanguagePanel(
            current = language,
            codes = languages,
            catalog = catalog,
            onPick = { code ->
                picker = false
                language = code
                scope.launch { io { setLanguage(code) }.onFailure { notice = it.message } }
            },
            onAdd = { picker = false; service.openApp("languages") },
            onClose = { picker = false },
        )
    }
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
    paused: Boolean,
    seconds: Long,
    kind: DictationKind,
    partial: String,
    language: String,
    showLanguage: Boolean,
    languages: List<String>,
    onLanguage: () -> Unit,
    onLongLanguage: () -> Unit,
    onPickLanguage: (String) -> Unit,
    packTip: Boolean,
    onPackTip: () -> Unit,
    onDismissPackTip: () -> Unit,
    onUndo: () -> Unit,
    onRewrite: (String, String) -> Unit,
) {
    val c = Tokens.colors
    Column(modifier.padding(horizontal = 6.dp), verticalArrangement = Arrangement.SpaceBetween) {
      Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.weight(1f).fillMaxHeight(), contentAlignment = Alignment.CenterStart) {
        when {
            recording -> Listening(kind, paused, seconds, partial, Modifier.fillMaxSize())
            notice != null -> Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxSize()) {
                Box(Modifier.size(8.dp).clip(RoundedCornerShape(50)).background(c.live))
                Spacer(Modifier.width(8.dp))
                Text(notice, style = HamsType.body, color = c.text, maxLines = 3, overflow = TextOverflow.Ellipsis)
            }
            last != null -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.CenterStart) {
                Text(
                    last.text, style = HamsType.body.copy(fontSize = 15.sp), color = c.text,
                    maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
            else -> Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxSize()) {
                Mark(size = 16, tint = c.voice)
                Spacer(Modifier.width(8.dp))
                Text(
                    "Hold the pad and speak. Hams cleans it up.",
                    style = HamsType.small, color = c.muted, maxLines = 2,
                )
            }
        }
        }
        if (showLanguage && !recording && languages.size < 2) {
            Spacer(Modifier.width(8.dp))
            LanguageChip(language, onLanguage, onLongLanguage)
        }
      }
        val chips = showLanguage && languages.size >= 2
        if (notice == null && !recording && (last != null || chips || packTip)) {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (chips) {
                    // One tap per language: a fixed language is both the most accurate and the fastest.
                    val current = language.takeIf { it != "auto" && it in languages } ?: "auto"
                    LangPill("AUTO", current == "auto", onLongLanguage) { onPickLanguage("auto") }
                    languages.forEach { code -> LangPill(code.uppercase(), current == code, onLongLanguage) { onPickLanguage(code) } }
                }
                if (packTip) PackTip(onPackTip, onDismissPackTip)
                if (last != null) {
                    if (chips || packTip) Spacer(Modifier.width(4.dp))
                    StripChip("Undo", icon = R.drawable.ic_undo, enabled = busy == null, onClick = onUndo)
                    REWRITES.forEach { (style, label) ->
                        StripChip(if (busy == label) "$label…" else label, enabled = busy == null) { onRewrite(style, label) }
                    }
                }
            }
        }
    }
}

/** While recording, the strip shows REC (or EDIT, or PAUSED), the time, and the words as they come. */
@Composable
private fun Listening(kind: DictationKind, paused: Boolean, seconds: Long, partial: String, modifier: Modifier) {
    val c = Tokens.colors
    val pulse = rememberInfiniteTransition(label = "dot")
    val blink by pulse.animateFloat(0.35f, 1f, infiniteRepeatable(tween(600), RepeatMode.Reverse), label = "alpha")
    Column(modifier, verticalArrangement = Arrangement.Center) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(8.dp).clip(CircleShape).background(if (paused) c.faint else c.live.copy(alpha = blink)))
            Spacer(Modifier.width(8.dp))
            Text(
                when {
                    paused -> "PAUSED"
                    kind == DictationKind.COMMAND -> "EDIT"
                    else -> "REC"
                },
                style = HamsType.label.copy(letterSpacing = 1.2.sp), color = c.text,
            )
            Spacer(Modifier.weight(1f))
            Text(
                "%02d:%02d".format(seconds / 60, seconds % 60),
                style = TextStyle(fontFamily = FontFamily.Monospace, fontSize = 20.sp, fontWeight = FontWeight.Light),
                color = c.text,
            )
        }
        if (partial.isNotBlank()) {
            Spacer(Modifier.height(4.dp))
            Text(partial, style = HamsType.small, color = c.muted, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
    }
}

/** Whole seconds recorded since `startedAt` while `active`, not counting time spent paused. */
@Composable
private fun elapsedSeconds(startedAt: Long, active: Boolean): Long {
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(startedAt, active) {
        while (isActive && active) {
            now = System.currentTimeMillis()
            delay(250)
        }
    }
    if (!active || startedAt <= 0) return 0
    val paused = Mic.pausedTotalMs + if (Mic.pausedSince > 0) now - Mic.pausedSince else 0
    return ((now - startedAt - paused) / 1000).coerceAtLeast(0)
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

/** With fewer than two languages there is nothing to switch: a tap opens Languages in the app, a long press lists them. */
@Composable
private fun LanguageChip(language: String, onTap: () -> Unit, onLongPress: () -> Unit) {
    val c = Tokens.colors
    val label = if (language == "auto" || language.isEmpty()) "AUTO" else language.uppercase()
    Box(
        Modifier
            .clip(RoundedCornerShape(50))
            .background(c.surface)
            .pointerInput(Unit) { detectTapGestures(onTap = { onTap() }, onLongPress = { onLongPress() }) }
            .semantics { contentDescription = "Dictation language: $label. Tap to add languages, hold for the list." }
            .padding(horizontal = 12.dp, vertical = 7.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, style = HamsType.label, color = c.text)
    }
}

/** One language in the strip. The selected one is filled; a long press lists all your languages. */
@Composable
private fun LangPill(label: String, selected: Boolean, onLongPress: () -> Unit, onClick: () -> Unit) {
    val c = Tokens.colors
    Box(
        Modifier
            .clip(RoundedCornerShape(50))
            .background(if (selected) c.primary else c.surface)
            .pointerInput(Unit) { detectTapGestures(onTap = { onClick() }, onLongPress = { onLongPress() }) }
            .semantics { contentDescription = "Dictate in $label" + if (selected) ", selected" else "" }
            .padding(horizontal = 12.dp, vertical = 7.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, style = HamsType.label, color = if (selected) c.primaryInk else c.text)
    }
}

/** Every language the user has, over the whole keyboard: tap one to dictate in it. */
@Composable
private fun LanguagePanel(
    current: String,
    codes: List<String>,
    catalog: Map<String, Lang>,
    onPick: (String) -> Unit,
    onAdd: () -> Unit,
    onClose: () -> Unit,
) {
    val c = Tokens.colors
    Column(
        Modifier.fillMaxSize().background(c.ground).pointerInput(Unit) { detectTapGestures { } }.padding(horizontal = 12.dp, vertical = 8.dp),
    ) {
        Row(Modifier.fillMaxWidth().height(40.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Dictate in", style = HamsType.section, color = c.text, modifier = Modifier.weight(1f))
            Box(
                Modifier.size(40.dp).clip(CircleShape).clickable(onClick = onClose).semantics { contentDescription = "Close" },
                contentAlignment = Alignment.Center,
            ) { HamsIcon(R.drawable.ic_close, tint = c.muted, size = 18.dp) }
        }
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
            LanguageRow("Auto", "Detect among your languages", current == "auto" || current !in codes) { onPick("auto") }
            codes.forEach { code ->
                val lang = catalog[code]
                LanguageRow(lang?.title ?: code.uppercase(), lang?.subtitle, current == code) { onPick(code) }
            }
            LanguageRow("Add a language", "Choose from every language Whisper knows", false, onClick = onAdd)
        }
    }
}

@Composable
private fun LanguageRow(title: String, subtitle: String?, selected: Boolean, onClick: () -> Unit) {
    val c = Tokens.colors
    Row(
        Modifier.fillMaxWidth().height(48.dp).clip(RoundedCornerShape(12.dp)).clickable(onClick = onClick).padding(horizontal = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = HamsType.body, color = c.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (subtitle != null) Text(subtitle, style = HamsType.small, color = c.muted, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        if (selected) HamsIcon(R.drawable.ic_check, tint = c.data, size = 18.dp)
    }
}

/** A one-time hint that a bigger speech pack suits the user's languages. The ✕ hides it for good. */
@Composable
private fun PackTip(onOpen: () -> Unit, onDismiss: () -> Unit) {
    val c = Tokens.colors
    Row(
        Modifier.clip(RoundedCornerShape(50)).background(c.surface),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            "Wrong language? Get the better pack",
            style = HamsType.label, color = c.text,
            modifier = Modifier.clickable(onClick = onOpen).padding(start = 12.dp, top = 7.dp, bottom = 7.dp, end = 6.dp),
        )
        Box(
            Modifier.clickable(onClick = onDismiss).semantics { contentDescription = "Dismiss" }.padding(horizontal = 8.dp, vertical = 7.dp),
        ) { HamsIcon(R.drawable.ic_close, tint = c.muted, size = 13.dp) }
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
    paused: Boolean,
    onNotice: (String) -> Unit,
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

    val density = LocalDensity.current
    val cancelDistance = with(density) { 96.dp.toPx() }
    var cancelled by remember { mutableStateOf(false) }

    val mode = when {
        !ready || working -> DotMode.Working
        blocker != null -> DotMode.Off
        recording && paused -> DotMode.Paused
        recording -> DotMode.Live
        else -> DotMode.Idle
    }
    // Ink for the dots: plain text colour; green while working; forest/mint when editing a selection.
    val tint = when {
        mode == DotMode.Working -> c.data
        pad == Pad.COMMAND -> if (c.dark) c.mint else c.primary
        else -> c.text
    }

    Box(
        modifier
            .clip(RoundedCornerShape(20.dp))
            .background(c.surface)
            .semantics { contentDescription = if (recording) "Recording. Release to insert." else "Hold to talk" },
    ) {
        // The touch surface is the whole card. The hands-free buttons sit above it as siblings,
        // so pressing them never reaches the hold gesture.
        Box(
            Modifier
                .fillMaxSize()
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
        ) {
            Column(Modifier.fillMaxSize().padding(horizontal = 16.dp, vertical = 10.dp)) {
                DotWave(mode, level, Modifier.fillMaxWidth().weight(1f), tint)
                Spacer(Modifier.height(4.dp))
                Row(Modifier.fillMaxWidth().height(44.dp), verticalAlignment = Alignment.CenterVertically) {
                    when {
                        blocker != null -> Text(
                            blocker.first, style = HamsType.small, color = c.muted,
                            maxLines = 2, overflow = TextOverflow.Ellipsis,
                        )
                        // Hands-free: the controls below sit on top of this row.
                        recording && locked -> Unit
                        recording -> Text(
                            if (pad == Pad.COMMAND) "Say the change, then let go" else "Let go to insert  ·  slide sideways to cancel",
                            style = HamsType.small, color = c.muted, maxLines = 2, overflow = TextOverflow.Ellipsis,
                        )
                        else -> Column(verticalArrangement = Arrangement.Center) {
                            Text(
                                when {
                                    working -> when (phase) {
                                        Phase.POLISHING -> "Polishing…"
                                        Phase.DELIVERING -> "Writing…"
                                        else -> "Transcribing…"
                                    }
                                    pad == Pad.COMMAND -> "Say how to change it"
                                    else -> "Hold to talk"
                                },
                                style = HamsType.bodyStrong, color = c.text, maxLines = 1,
                            )
                            if (!working) {
                                Text(
                                    if (pad == Pad.COMMAND) "\"friendlier\", \"in Malayalam\"" else "Double-tap for hands-free",
                                    style = HamsType.small, color = c.muted, maxLines = 1, overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                    }
                }
            }
        }
        if (recording && locked && blocker == null) {
            Row(
                Modifier.align(Alignment.BottomCenter).fillMaxWidth().padding(horizontal = 12.dp).padding(bottom = 10.dp).height(44.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                RoundButton(R.drawable.ic_close, "Cancel recording", filled = false) {
                    view.performHapticFeedback(HapticFeedbackConstants.REJECT)
                    engine?.cancel()
                }
                Box(
                    Modifier
                        .weight(1f)
                        .fillMaxHeight()
                        .clip(CircleShape)
                        .background(c.voice)
                        .semantics { contentDescription = "Stop listening and insert" }
                        .clickable {
                            view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_RELEASE)
                            engine?.stop()
                        },
                    contentAlignment = Alignment.Center,
                ) { Text("Stop listening", style = HamsType.bodyStrong, color = c.voiceInk) }
                RoundButton(
                    if (paused) R.drawable.ic_mic else null,
                    if (paused) "Resume recording" else "Pause recording", filled = false,
                ) {
                    view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                    if (paused) {
                        if (!Mic.resume()) onNotice("Couldn't reopen the microphone. Another app may be using it.")
                    } else {
                        Mic.pause()
                    }
                }
            }
        }
    }
}

/** A round action on the recorder. A null `icon` draws the pause glyph (two bars). */
@Composable
private fun RoundButton(icon: Int?, description: String, filled: Boolean, onClick: () -> Unit) {
    val c = Tokens.colors
    Box(
        Modifier
            .size(44.dp)
            .clip(CircleShape)
            .background(if (filled) c.primary else c.control)
            .semantics { contentDescription = description }
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        if (icon != null) {
            HamsIcon(icon, tint = if (filled) c.primaryInk else c.text, size = 22.dp)
        } else {
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                repeat(2) { Box(Modifier.width(4.dp).height(16.dp).clip(RoundedCornerShape(2.dp)).background(c.text)) }
            }
        }
    }
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
