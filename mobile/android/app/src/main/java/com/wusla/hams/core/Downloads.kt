package com.wusla.hams.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Speech-pack downloads, owned by the process and not by a screen.
 *
 * They used to run in the composition's scope, so rotating the phone or
 * leaving the screen threw away the progress while the engine kept writing the
 * file; tapping Download again then started a second writer on the same
 * `.part`. Here a download keeps going (and keeps being reported) whichever
 * screen is showing, and a second start for the same pack is ignored.
 */
object Downloads {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /** Pack id → progress 0..1 while it downloads. */
    val progress = mutableStateMapOf<String, Float>()

    /** Pack id → why the last attempt failed. Cleared when it is tried again. */
    val failures = mutableStateMapOf<String, String>()

    /** Bumped whenever a download ends, so screens reload what the engine has. */
    var finished by mutableIntStateOf(0)
        private set

    /**
     * Download a pack. The engine continues a partly downloaded file, so trying
     * again after a failure resumes. With `thenUse`, switch to the pack once it
     * is on the phone (not before: the keyboard needs a working one meanwhile).
     */
    fun start(id: String, thenUse: Boolean = false) {
        if (progress.containsKey(id)) return
        failures.remove(id)
        progress[id] = 0f
        scope.launch {
            val listener = object : DownloadListener {
                override fun onProgress(fraction: Float) {
                    progress[id] = fraction
                }
            }
            val result = io { downloadModel(id, listener) }
            result.onFailure { failures[id] = it.message ?: "The download stopped." }
            if (result.isSuccess && thenUse) {
                io { SettingsDoc.parse(settingsJson()).edit { put("model", id) } }
                    .onSuccess { next -> io { saveSettingsJson(next.json.toString()) } }
            }
            withContext(Dispatchers.Main) {
                progress.remove(id)
                finished++
            }
        }
    }
}
