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

/**
 * The microphone, open only while a dictation records: the engine calls
 * [start] when recording begins and [stop] when it ends. Frames go straight to
 * the engine as 16-bit PCM bytes at 16 kHz (Whisper's rate, so no resampling).
 */
object Mic {
    private const val TAG = "HamsMic"
    private const val RATE = 16_000

    private lateinit var app: Context
    private var record: AudioRecord? = null
    private var thread: Thread? = null
    @Volatile private var running = false

    fun init(context: Context) {
        app = context.applicationContext
    }

    fun hasPermission(context: Context = app): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED

    @SuppressLint("MissingPermission")
    @Synchronized
    fun start(sink: (ByteArray, Int) -> Unit): Boolean {
        stop()
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
