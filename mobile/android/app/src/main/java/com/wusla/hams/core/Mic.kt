package com.wusla.hams.core

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.util.Log
import androidx.core.content.ContextCompat
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * The microphone, open only while a dictation records: the engine calls
 * [start] when recording begins and [stop] when it ends. Frames go straight to
 * the engine as 16-bit PCM bytes at 16 kHz (Whisper's rate, so no resampling).
 *
 * [pause] closes the microphone (the system indicator goes off) while the engine
 * keeps the audio it already has; [resume] reopens it and the recording carries on.
 */
object Mic {
    private const val TAG = "HamsMic"
    private const val RATE = 16_000

    private lateinit var app: Context
    private var record: AudioRecord? = null
    private var thread: Thread? = null
    @Volatile private var running = false
    private var sink: ((ByteArray, Int) -> Unit)? = null

    private val _paused = MutableStateFlow(false)
    val paused: StateFlow<Boolean> = _paused.asStateFlow()
    /** Milliseconds spent paused in this recording, and when the current pause began (0 if not paused). */
    @Volatile var pausedTotalMs = 0L
        private set
    @Volatile var pausedSince = 0L
        private set

    fun init(context: Context) {
        app = context.applicationContext
    }

    fun hasPermission(context: Context = app): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED

    @Synchronized
    fun start(sink: (ByteArray, Int) -> Unit): Boolean {
        release()
        this.sink = sink
        pausedTotalMs = 0
        pausedSince = 0
        _paused.value = false
        return open(sink)
    }

    /** Close the microphone but keep the recording. */
    @Synchronized
    fun pause() {
        if (record == null || _paused.value) return
        release()
        pausedSince = System.currentTimeMillis()
        _paused.value = true
    }

    /** Reopen the microphone after [pause]. False if it can't (another app holds it): stays paused. */
    @Synchronized
    fun resume(): Boolean {
        val sink = sink ?: return false
        if (!_paused.value) return true
        if (!open(sink)) return false
        pausedTotalMs += System.currentTimeMillis() - pausedSince
        pausedSince = 0
        _paused.value = false
        return true
    }

    @SuppressLint("MissingPermission")
    private fun open(sink: (ByteArray, Int) -> Unit): Boolean {
        if (!hasPermission()) return false
        val minBuffer = AudioRecord.getMinBufferSize(RATE, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT)
        if (minBuffer <= 0) return false
        val recorder = try {
            AudioRecord(
                MediaRecorder.AudioSource.VOICE_RECOGNITION, RATE,
                AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT,
                maxOf(minBuffer * 2, RATE / 5 * 2),
            )
        } catch (t: Throwable) {
            Log.w(TAG, "AudioRecord failed", t)
            return false
        }
        if (recorder.state != AudioRecord.STATE_INITIALIZED) {
            recorder.release()
            return false
        }
        try {
            recorder.startRecording()
        } catch (t: IllegalStateException) {
            recorder.release()
            return false
        }
        if (recorder.recordingState != AudioRecord.RECORDSTATE_RECORDING) {
            // Another app holds the microphone.
            recorder.release()
            return false
        }
        record = recorder
        running = true
        thread = Thread({
            // 50 ms reads: smooth level meter, little overhead.
            val buffer = ByteArray(RATE / 20 * 2)
            while (running) {
                val read = recorder.read(buffer, 0, buffer.size)
                if (read > 0) sink(if (read == buffer.size) buffer.copyOf() else buffer.copyOf(read), RATE)
                else if (read < 0) break
            }
        }, "hams-mic").apply {
            priority = Thread.MAX_PRIORITY
            start()
        }
        return true
    }

    @Synchronized
    fun stop() {
        release()
        sink = null
        pausedSince = 0
        _paused.value = false
    }

    private fun release() {
        running = false
        thread?.join(300)
        thread = null
        record?.let {
            try {
                it.stop()
            } catch (_: IllegalStateException) {
            }
            it.release()
        }
        record = null
    }
}
