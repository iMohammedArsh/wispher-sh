package com.wusla.hams

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.wusla.hams.core.DictationStatus
import com.wusla.hams.core.DownloadListener
import com.wusla.hams.core.FieldInfo
import com.wusla.hams.core.FieldKind
import com.wusla.hams.core.HamsEngine
import com.wusla.hams.core.Host
import com.wusla.hams.core.Pad
import com.wusla.hams.core.Phase
import com.wusla.hams.core.SecretVault
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.concurrent.CopyOnWriteArrayList

/**
 * The real engine on a real (or emulated) phone: JNA loads the .so, Whisper
 * transcribes recorded speech fed exactly like the microphone feeds it, and the
 * text lands in a field. Downloads the phone model (57 MB) on first run.
 */
@RunWith(AndroidJUnit4::class)
class EngineOnDeviceTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext
    private val testContext = InstrumentationRegistry.getInstrumentation().context

    private class FakeField : Host {
        var engine: HamsEngine? = null
        var pcm: ByteArray = ByteArray(0)
        /** Pause after each 50 ms buffer: 50 is real time, like a microphone. */
        var paceMs = 10L
        val text = StringBuilder()
        val statuses = CopyOnWriteArrayList<DictationStatus>()

        override fun micStart(): Boolean {
            // Like AudioRecord: 50 ms buffers arriving in real time.
            Thread {
                var offset = 0
                val chunk = 16_000 / 20 * 2
                while (offset < pcm.size) {
                    val end = minOf(offset + chunk, pcm.size)
                    engine?.pushAudioPcm16(pcm.copyOfRange(offset, end), 16_000u)
                    offset = end
                    Thread.sleep(paceMs)
                }
            }.start()
            return true
        }
        override fun micStop() {}
        override fun field() = FieldInfo(1uL, "com.example.notes", "Notes", FieldKind.TEXT)
        override fun textBeforeCursor(maxUtf16: UInt) = text.takeLast(maxUtf16.toInt()).toString()
        override fun selectedText() = ""
        override fun fieldText() = text.toString()
        override fun commit(token: ULong, text: String): Boolean {
            this.text.append(text)
            return true
        }
        override fun deleteBeforeCursor(token: ULong, count: UInt): Boolean {
            text.setLength(maxOf(0, text.length - count.toInt()))
            return true
        }
        override fun copy(text: String) {}
        override fun onStatus(status: DictationStatus) {
            statuses.add(status)
        }
        override fun onLevel(level: Float) {}
        override fun onPartial(text: String) {}
        override fun onDataChanged() {}
        override fun log(level: UByte, message: String) {}
    }

    /** The samples of a 16-bit WAV: the bytes after the "data" chunk header. */
    private fun pcmOf(wav: ByteArray): ByteArray {
        var pos = 12
        while (pos + 8 <= wav.size) {
            val size = (wav[pos + 4].toInt() and 0xff) or ((wav[pos + 5].toInt() and 0xff) shl 8) or
                ((wav[pos + 6].toInt() and 0xff) shl 16) or ((wav[pos + 7].toInt() and 0xff) shl 24)
            if (String(wav, pos, 4, Charsets.US_ASCII) == "data") return wav.copyOfRange(pos + 8, minOf(wav.size, pos + 8 + size))
            pos += 8 + size
        }
        error("no data chunk")
    }

    /** `copies` of `pcm` with `gapMs` of silence between them: a long dictation with pauses to cut at. */
    private fun repeated(pcm: ByteArray, copies: Int, gapMs: Int): ByteArray {
        val gap = ByteArray(16_000 * 2 * gapMs / 1000)
        val out = java.io.ByteArrayOutputStream()
        repeat(copies) {
            out.write(pcm)
            out.write(gap)
        }
        return out.toByteArray()
    }

    private object NoVault : SecretVault {
        override fun get(account: String): String? = null
        override fun set(account: String, value: String) = false
    }

    @Test
    fun holdToTalkTranscribesOnTheDeviceAndInserts() {
        // Shares the app's models folder (the app's engine sets it; the app starts
        // it in the background, so wait for it), with its own database.
        com.wusla.hams.core.Hams.init(context)
        val dir = File(context.cacheDir, "engine-test-${System.nanoTime()}").apply { mkdirs() }
        val host = FakeField()
        val engine = HamsEngine(dir.absolutePath, host, NoVault)
        host.engine = engine
        host.pcm = pcmOf(testContext.assets.open("dictation.wav").use { it.readBytes() })

        engine.downloadModel("base-q5_1", object : DownloadListener {
            override fun onProgress(fraction: Float) {}
        })
        assertTrue("model downloaded", engine.modelReady())

        // Twice: the first dictation may wait for the model to load; the second shows steady-state speed.
        for (round in 1..2) {
            host.statuses.clear()
            host.text.setLength(0)
            engine.press(Pad.DICTATE)
            // Hold for the length of the speech plus a little.
            Thread.sleep(host.pcm.size / 32L + 700)
            val released = System.currentTimeMillis()
            engine.release(Pad.DICTATE)

            val deadline = System.currentTimeMillis() + 90_000
            while (host.statuses.none { it.phase == Phase.DONE || it.phase == Phase.ERROR }) {
                assertTrue("timed out; statuses ${host.statuses}", System.currentTimeMillis() < deadline)
                Thread.sleep(20)
            }
            val error = host.statuses.firstOrNull { it.phase == Phase.ERROR }
            assertTrue("error: ${error?.message}", error == null)
            val written = host.text.toString()
            android.util.Log.i("HamsTest", "Round $round: ${System.currentTimeMillis() - released} ms after release: $written")
            assertTrue("got: $written", written.lowercase().contains("dictation test"))
            assertTrue("filler removed: $written", !written.lowercase().startsWith("um"))
        }

    }

    /**
     * A dictation of about a minute used to be transcribed only after stop. Now it is
     * transcribed in ~30 s pieces while recording, so what is left after stop is the
     * last piece. Run with the live preview off and on: the preview must not slow the
     * final text down either. Real-time pacing, so this takes about two minutes.
     * Compare "ms after release" with the length of the speech in logcat (tag HamsTest).
     */
    @Test
    fun aLongDictationIsReadyShortlyAfterStopWithAndWithoutLivePreview() {
        com.wusla.hams.core.Hams.init(context)
        val dir = File(context.cacheDir, "engine-long-${System.nanoTime()}").apply { mkdirs() }
        val host = FakeField()
        val engine = HamsEngine(dir.absolutePath, host, NoVault)
        host.engine = engine
        host.paceMs = 50
        val one = pcmOf(testContext.assets.open("dictation.wav").use { it.readBytes() })
        host.pcm = repeated(one, copies = 14, gapMs = 700)
        val speechSeconds = host.pcm.size / 32_000
        engine.downloadModel("base-q5_1", object : DownloadListener {
            override fun onProgress(fraction: Float) {}
        })
        assertTrue("model downloaded", engine.modelReady())
        // A short dictation first, so the model is loaded and warm for both rounds.
        engine.press(Pad.DICTATE); Thread.sleep(1_000); engine.release(Pad.DICTATE)
        Thread.sleep(8_000)

        for (preview in listOf(false, true)) {
            val settings = org.json.JSONObject(engine.settingsJson())
            settings.getJSONObject("settings").put("live_preview", preview)
            engine.saveSettingsJson(settings.getJSONObject("settings").toString())
            host.statuses.clear()
            host.text.setLength(0)
            engine.press(Pad.DICTATE)
            Thread.sleep(host.pcm.size / 32L + 300)
            val released = System.currentTimeMillis()
            engine.release(Pad.DICTATE)

            val deadline = System.currentTimeMillis() + 120_000
            while (host.statuses.none { it.phase == Phase.DONE || it.phase == Phase.ERROR }) {
                assertTrue("timed out; statuses ${host.statuses}", System.currentTimeMillis() < deadline)
                Thread.sleep(20)
            }
            val error = host.statuses.firstOrNull { it.phase == Phase.ERROR }
            assertTrue("error: ${error?.message}", error == null)
            val waited = System.currentTimeMillis() - released
            val written = host.text.toString()
            android.util.Log.i("HamsTest", "Long dictation, live preview $preview: $waited ms after release for $speechSeconds s of speech; ${written.length} characters")
            // Every repeat of the sentence made it into the text, across the chunk cuts.
            val heard = Regex("dictation test", RegexOption.IGNORE_CASE).findAll(written).count()
            assertTrue("heard $heard of 14 repeats: $written", heard >= 12)
            assertTrue("took $waited ms after release", waited < 60_000)
        }
    }
}
