package com.wusla.hams.ui

import android.view.HapticFeedbackConstants
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalView
import com.wusla.hams.core.FieldBridge
import com.wusla.hams.core.FieldInfo
import com.wusla.hams.core.FieldKind
import com.wusla.hams.core.Hams
import com.wusla.hams.core.Pad
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * A text field inside the app that the engine can type into, so the orb on
 * Home and in onboarding dictates right there, without switching keyboards.
 * It goes through the normal pipeline (clean-up, dictionary, polish, history);
 * only the "field" is ours instead of another app's.
 */
object TryField : FieldBridge {
    // Far above the keyboard's tokens (which count up from 1), so the two never collide.
    private var nextToken = 1L shl 40
    @Volatile private var info = FieldInfo(0uL, "", "Hams", FieldKind.TEXT)

    private val _text = MutableStateFlow("")
    /** Everything dictated into the in-app field since it was last cleared. */
    val text: StateFlow<String> = _text.asStateFlow()

    /** Point the engine at this field (on press, before recording starts). */
    fun attach(packageName: String) {
        if (Hams.field !== this || info.token == 0uL) {
            info = FieldInfo((nextToken++).toULong(), packageName, "Hams", FieldKind.TEXT)
        }
        Hams.field = this
    }

    /** Stop being the engine's field (the screen with the orb went away). */
    fun detach() {
        if (Hams.field === this) Hams.field = null
        info = info.copy(token = 0uL)
    }

    fun clear() {
        _text.value = ""
    }

    override fun info(): FieldInfo = info
    override fun textBeforeCursor(max: Int): String = _text.value.takeLast(max)
    override fun selectedText(): String = ""
    override fun fieldText(): String = _text.value

    override fun commit(token: Long, text: String): Boolean {
        if (token.toULong() != info.token || info.token == 0uL) return false
        _text.value += text
        return true
    }

    override fun deleteBeforeCursor(token: Long, count: Int): Boolean {
        if (token.toULong() != info.token || info.token == 0uL) return false
        val current = _text.value
        if (count > current.length) return false
        _text.value = current.dropLast(count)
        return true
    }
}

/** Detach the in-app field when the screen showing the orb leaves. */
@Composable
fun TryFieldLifecycle() {
    DisposableEffect(Unit) { onDispose { TryField.detach() } }
}

/**
 * Hold to talk into [TryField]: press starts, lifting the finger stops. A quick
 * double tap locks hands-free (the engine's own tap timing), exactly like the
 * keyboard's mic pad.
 */
fun Modifier.holdToTalk(enabled: Boolean, onBlocked: () -> Unit = {}): Modifier = composed {
    val view = LocalView.current
    val focus = LocalFocusManager.current
    val packageName = view.context.packageName
    pointerInput(enabled) {
        awaitEachGesture {
            awaitFirstDown(requireUnconsumed = false)
            val engine = Hams.engineOrNull
            if (!enabled || engine == null) {
                waitUntilUp()
                onBlocked()
                return@awaitEachGesture
            }
            focus.clearFocus()
            TryField.attach(packageName)
            view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
            engine.press(Pad.DICTATE)
            waitUntilUp()
            view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_RELEASE)
            engine.release(Pad.DICTATE)
        }
    }
}

private suspend fun androidx.compose.ui.input.pointer.AwaitPointerEventScope.waitUntilUp() {
    while (true) {
        if (awaitPointerEvent().changes.none { it.pressed }) return
    }
}
