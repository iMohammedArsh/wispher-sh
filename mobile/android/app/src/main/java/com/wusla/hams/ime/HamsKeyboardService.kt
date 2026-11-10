package com.wusla.hams.ime

import android.content.Intent
import android.inputmethodservice.InputMethodService
import android.os.Handler
import android.os.Looper
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.ExtractedTextRequest
import android.view.inputmethod.InputMethodManager
import androidx.compose.ui.platform.ComposeView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.setViewTreeLifecycleOwner
import androidx.lifecycle.setViewTreeViewModelStoreOwner
import androidx.savedstate.SavedStateRegistry
import androidx.savedstate.SavedStateRegistryController
import androidx.savedstate.SavedStateRegistryOwner
import androidx.savedstate.setViewTreeSavedStateRegistryOwner
import com.wusla.hams.core.FieldBridge
import com.wusla.hams.core.FieldInfo
import com.wusla.hams.core.FieldKind
import com.wusla.hams.core.Hams
import com.wusla.hams.core.Mic
import com.wusla.hams.ui.MainActivity
import com.wusla.hams.ui.theme.HamsTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

/**
 * The Hams voice keyboard. It owns no dictation state: the engine's controller
 * does. This class is the bridge to the text field (the engine's "desktop")
 * and hosts the Compose keyboard.
 *
 * Every field the keyboard attaches to gets a new token. The engine notes the
 * token when recording starts and inserts only if the same field is still
 * attached, so switching fields mid-dictation copies instead of typing in the
 * wrong place.
 */
class HamsKeyboardService :
    InputMethodService(), LifecycleOwner, SavedStateRegistryOwner, ViewModelStoreOwner, FieldBridge {

    private val lifecycleRegistry = LifecycleRegistry(this)
    private val savedState = SavedStateRegistryController.create(this)
    override val lifecycle: Lifecycle get() = lifecycleRegistry
    override val savedStateRegistry: SavedStateRegistry get() = savedState.savedStateRegistry
    override val viewModelStore = ViewModelStore()

    private val main = Handler(Looper.getMainLooper())
    private var nextToken = 1L
    private var fieldKey: String? = null
    @Volatile private var field = FieldInfo(0uL, "", "", FieldKind.TEXT)

    /** What the keyboard UI shows about the current field. */
    data class Editor(
        val enter: FieldKinds.Enter = FieldKinds.Enter.NEWLINE,
        val kind: FieldKind = FieldKind.TEXT,
        val hasSelection: Boolean = false,
        val micAllowed: Boolean = true,
        val modelReady: Boolean = true,
        val engineError: String? = null,
    )

    private val _editor = MutableStateFlow(Editor())
    val editor: StateFlow<Editor> = _editor.asStateFlow()

    override fun onCreate() {
        super.onCreate()
        savedState.performAttach()
        savedState.performRestore(null)
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_CREATE)
        // The app process may have been started for the keyboard: never wait for
        // the engine on the main thread (that would also freeze the app's own
        // window). The keyboard shows "Starting…" until it's ready.
        Hams.initAsync(applicationContext)
        scope.launch {
            Hams.ready.first { it }
            refreshEngineState()
        }
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var inputViewShown = false

    /** What the keyboard needs to know from the engine: is it running, mic, model. */
    private fun refreshEngineState() {
        val engine = Hams.engineOrNull
        _editor.value = _editor.value.copy(
            micAllowed = Mic.hasPermission(this),
            modelReady = engine?.modelReady() ?: false,
            engineError = Hams.startError,
        )
        if (inputViewShown) Hams.warmUpOnce()
    }

    override fun onCreateInputView(): View {
        // Compose needs these owners on the IME window, which an InputMethodService doesn't provide.
        window.window?.decorView?.let { decor ->
            decor.setViewTreeLifecycleOwner(this)
            decor.setViewTreeSavedStateRegistryOwner(this)
            decor.setViewTreeViewModelStoreOwner(this)
        }
        return ComposeView(this).apply {
            setViewTreeLifecycleOwner(this@HamsKeyboardService)
            setViewTreeSavedStateRegistryOwner(this@HamsKeyboardService)
            setViewTreeViewModelStoreOwner(this@HamsKeyboardService)
            setContent { HamsTheme { KeyboardScreen(this@HamsKeyboardService) } }
        }
    }

    // ---- field lifecycle ---------------------------------------------------------------

    override fun onStartInput(attribute: EditorInfo?, restarting: Boolean) {
        super.onStartInput(attribute, restarting)
        val info = attribute ?: return
        val pkg = info.packageName.orEmpty()
        // Apps restart input on the same field (after a commit, on rotation); keep its token then.
        val key = "$pkg:${info.fieldId}:${info.inputType}"
        if (key != fieldKey || field.token == 0uL) {
            fieldKey = key
            field = FieldInfo(nextToken++.toULong(), pkg, appLabel(pkg), FieldKinds.kindOf(info.inputType))
        }
        Hams.field = this
        _editor.value = _editor.value.copy(
            enter = FieldKinds.enterOf(info.inputType, info.imeOptions),
            kind = field.kind,
            hasSelection = info.initialSelStart != info.initialSelEnd,
        )
    }

    override fun onStartInputView(info: EditorInfo?, restarting: Boolean) {
        super.onStartInputView(info, restarting)
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_RESUME)
        inputViewShown = true
        refreshEngineState()
    }

    override fun onFinishInputView(finishingInput: Boolean) {
        // The keyboard went away mid-dictation (app switched, field closed):
        // don't keep the mic open for nobody.
        Hams.engineOrNull?.let { if (it.isRecording()) it.cancel() }
        inputViewShown = false
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_PAUSE)
        super.onFinishInputView(finishingInput)
    }

    override fun onFinishInput() {
        super.onFinishInput()
        // No field: anything still being transcribed is copied, not typed.
        field = field.copy(token = 0uL)
        fieldKey = null
    }

    override fun onUpdateSelection(oldSelStart: Int, oldSelEnd: Int, newSelStart: Int, newSelEnd: Int, candidatesStart: Int, candidatesEnd: Int) {
        super.onUpdateSelection(oldSelStart, oldSelEnd, newSelStart, newSelEnd, candidatesStart, candidatesEnd)
        val selected = newSelStart != newSelEnd
        if (selected != _editor.value.hasSelection) _editor.value = _editor.value.copy(hasSelection = selected)
    }

    override fun onDestroy() {
        if (Hams.field === this) Hams.field = null
        scope.cancel()
        lifecycleRegistry.handleLifecycleEvent(Lifecycle.Event.ON_DESTROY)
        viewModelStore.clear()
        super.onDestroy()
    }

    // The keyboard is never fullscreen: it's a strip under the app, not an editor.
    override fun onEvaluateFullscreenMode(): Boolean = false

    // ---- FieldBridge (called by the engine on its threads) ---------------------------------

    override fun info(): FieldInfo = field

    override fun textBeforeCursor(max: Int): String =
        onMain("") { currentInputConnection?.getTextBeforeCursor(max, 0)?.toString() ?: "" }

    override fun selectedText(): String =
        onMain("") { currentInputConnection?.getSelectedText(0)?.toString() ?: "" }

    override fun fieldText(): String = onMain("") {
        currentInputConnection?.getExtractedText(ExtractedTextRequest(), 0)?.text?.toString() ?: ""
    }

    override fun commit(token: Long, text: String): Boolean = onMain(false) {
        val ic = currentInputConnection
        if (token.toULong() != field.token || ic == null) return@onMain false
        ic.beginBatchEdit()
        val ok = ic.commitText(text, 1)
        ic.endBatchEdit()
        ok
    }

    override fun deleteBeforeCursor(token: Long, count: Int): Boolean = onMain(false) {
        val ic = currentInputConnection
        if (token.toULong() != field.token || ic == null) return@onMain false
        ic.deleteSurroundingText(count, 0)
    }

    /** InputConnection calls belong on the main thread; the engine waits briefly for the answer. */
    private fun <T> onMain(fallback: T, block: () -> T): T {
        if (Looper.myLooper() == Looper.getMainLooper()) return block()
        val result = AtomicReference(fallback)
        val latch = CountDownLatch(1)
        main.post {
            try {
                result.set(block())
            } finally {
                latch.countDown()
            }
        }
        latch.await(1_500, TimeUnit.MILLISECONDS)
        return result.get()
    }

    // ---- keys -------------------------------------------------------------------------

    fun type(text: String) {
        currentInputConnection?.commitText(text, 1)
    }

    fun backspace() {
        val ic = currentInputConnection ?: return
        if (!ic.getSelectedText(0).isNullOrEmpty()) ic.commitText("", 1) else sendDownUpKeyEvents(KeyEvent.KEYCODE_DEL)
    }

    /** Delete the word before the cursor (long-press backspace). */
    fun deleteWord() {
        val ic = currentInputConnection ?: return
        val before = ic.getTextBeforeCursor(64, 0)?.toString() ?: return
        val trimmed = before.trimEnd()
        val start = trimmed.lastIndexOfAny(charArrayOf(' ', '\n')) + 1
        val count = before.length - start
        if (count > 0) ic.deleteSurroundingText(count, 0)
    }

    fun enter() {
        val ic = currentInputConnection ?: return
        val enter = _editor.value.enter
        if (enter == FieldKinds.Enter.NEWLINE) ic.commitText("\n", 1) else ic.performEditorAction(FieldKinds.actionOf(enter))
    }

    /** Back to the user's typing keyboard. */
    fun switchKeyboard() {
        if (!switchToPreviousInputMethod()) {
            if (!switchToNextInputMethod(false)) showPicker()
        }
    }

    fun showPicker() {
        getSystemService(InputMethodManager::class.java)?.showInputMethodPicker()
    }

    /** Open the app (permissions, model download). */
    fun openApp(section: String? = null) {
        startActivity(
            Intent(this, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
                .putExtra(MainActivity.EXTRA_SECTION, section),
        )
    }

    private val labels = HashMap<String, String>()

    private fun appLabel(pkg: String): String {
        if (pkg.isEmpty()) return ""
        return labels.getOrPut(pkg) {
            try {
                packageManager.getApplicationLabel(packageManager.getApplicationInfo(pkg, 0)).toString()
            } catch (_: Exception) {
                pkg
            }
        }
    }
}
