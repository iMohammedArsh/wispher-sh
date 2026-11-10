package com.wusla.hams.core

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * One engine per process, shared by the keyboard and the app screens. The
 * engine is the Rust `hams-mobile` crate: transcription, clean-up, dictionary,
 * styles, AI polish and history are the same code the desktop app runs.
 */
object Hams {
    private const val TAG = "Hams"

    @Volatile
    var engineOrNull: HamsEngine? = null
        private set

    /** Why the engine couldn't start (shown on Home), or null. */
    var startError: String? = null
        private set

    val engine: HamsEngine
        get() = engineOrNull ?: error(startError ?: "Hams engine not started")

    private lateinit var app: Context
    private val main = Handler(Looper.getMainLooper())

    /** What the keyboard is attached to; set by the keyboard service. */
    @Volatile
    var field: FieldBridge? = null

    private val _status = MutableStateFlow(idleStatus())
    /** The current dictation phase, for the keyboard and the app. */
    val status: StateFlow<DictationStatus> = _status.asStateFlow()

    private val _level = MutableStateFlow(0f)
    val level: StateFlow<Float> = _level.asStateFlow()

    private val _partial = MutableStateFlow("")
    val partial: StateFlow<String> = _partial.asStateFlow()

    private val _dataVersion = MutableStateFlow(0)
    /** Bumped whenever history, the dictionary or settings change. */
    val dataVersion: StateFlow<Int> = _dataVersion.asStateFlow()

    private val _ready = MutableStateFlow(false)
    /** True once [init] has finished (engine started, or failed with [startError]). */
    val ready: StateFlow<Boolean> = _ready.asStateFlow()

    /**
     * Start the engine on a background thread, so the app can draw its first
     * frame while the native library loads (about 2-4 s on a cold start).
     * Anything that needs the engine right away (the keyboard) calls [init],
     * which waits for this.
     */
    fun initAsync(context: Context) {
        val appContext = context.applicationContext
        Thread({ init(appContext) }, "hams-init").start()
    }

    @Synchronized
    fun init(context: Context) {
        if (_ready.value) return
        try {
            start(context)
        } finally {
            _ready.value = true
        }
    }

    private fun start(context: Context) {
        app = context.applicationContext
        Mic.init(app)
        chooseEngineBuild()
        try {
            engineOrNull = HamsEngine(app.filesDir.absolutePath, Router, KeystoreVault(app))
            startError = null
        } catch (t: Throwable) {
            // UnsatisfiedLinkError (missing .so) or a database error: keep the
            // app usable enough to show what went wrong.
            Log.e(TAG, "Engine failed to start", t)
            startError = t.message ?: t.javaClass.simpleName
        }
    }

    @Volatile private var warmed = false

    /** Load the speech model ahead of the first dictation (once, until memory is trimmed). */
    fun warmUpOnce() {
        val engine = engineOrNull ?: return
        if (warmed || !engine.modelReady()) return
        warmed = true
        engine.warmUp()
    }

    fun releaseModel() {
        warmed = false
        engineOrNull?.releaseModel()
    }

    /**
     * The arm64 engine is built for ARMv8.2 dot-product instructions (several
     * times faster Whisper). Phones without them load the baseline build.
     */
    private fun chooseEngineBuild() {
        val arm64 = android.os.Build.SUPPORTED_ABIS.firstOrNull() == "arm64-v8a"
        if (arm64 && !CpuFeatures.hasDotProduct()) {
            Log.i(TAG, "CPU has no dot-product instructions; using the compatible engine build")
            System.setProperty("uniffi.component.hams_mobile.libraryOverride", "hams_mobile_compat")
        }
    }

    fun notifyDataChanged() {
        _dataVersion.value += 1
    }

    /** Engine → phone. Called on engine threads. */
    private object Router : Host {
        override fun micStart(): Boolean = Mic.start { bytes, rate -> engineOrNull?.pushAudioPcm16(bytes, rate.toUInt()) }
        override fun micStop() = Mic.stop()

        override fun field(): FieldInfo = field?.info() ?: NO_FIELD
        override fun textBeforeCursor(maxUtf16: UInt): String = field?.textBeforeCursor(maxUtf16.toInt()) ?: ""
        override fun selectedText(): String = field?.selectedText() ?: ""
        override fun fieldText(): String = field?.fieldText() ?: ""
        override fun commit(token: ULong, text: String): Boolean = field?.commit(token.toLong(), text) ?: false
        override fun deleteBeforeCursor(token: ULong, count: UInt): Boolean =
            field?.deleteBeforeCursor(token.toLong(), count.toInt()) ?: false

        override fun copy(text: String) {
            main.post {
                val clipboard = app.getSystemService(ClipboardManager::class.java)
                clipboard?.setPrimaryClip(ClipData.newPlainText("Hams", text))
            }
        }

        override fun onStatus(status: DictationStatus) {
            _status.value = status
            if (status.phase != Phase.RECORDING) {
                _level.value = 0f
                _partial.value = ""
            }
        }

        override fun onLevel(level: Float) {
            _level.value = level
        }

        override fun onPartial(text: String) {
            _partial.value = text
        }

        override fun onDataChanged() = notifyDataChanged()

        override fun log(level: UByte, message: String) {
            val priority = when (level.toInt()) {
                1 -> Log.ERROR
                2 -> Log.WARN
                3 -> Log.INFO
                else -> Log.DEBUG
            }
            Log.println(priority, TAG, message)
        }
    }

    private val NO_FIELD = FieldInfo(0uL, "", "", FieldKind.TEXT)

    fun idleStatus() = DictationStatus(
        phase = Phase.IDLE, kind = DictationKind.DICTATE, locked = false, startedAt = 0L,
        message = null, text = null, app = null, preview = false,
    )
}

/** The text field under the keyboard. Implemented by the keyboard service. */
interface FieldBridge {
    fun info(): FieldInfo
    fun textBeforeCursor(max: Int): String
    fun selectedText(): String
    fun fieldText(): String
    fun commit(token: Long, text: String): Boolean
    fun deleteBeforeCursor(token: Long, count: Int): Boolean
}
