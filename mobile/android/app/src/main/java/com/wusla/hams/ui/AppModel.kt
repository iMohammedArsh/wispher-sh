package com.wusla.hams.ui

import android.content.Context
import android.provider.Settings
import android.view.inputmethod.InputMethodManager
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.wusla.hams.core.DictionaryEntry
import com.wusla.hams.core.DownloadListener
import com.wusla.hams.core.Hams
import com.wusla.hams.core.HistoryItem
import com.wusla.hams.core.Mic
import com.wusla.hams.core.ModelStatus
import com.wusla.hams.core.Parse
import com.wusla.hams.core.SettingsDoc
import com.wusla.hams.core.Stats
import com.wusla.hams.core.Snippet
import com.wusla.hams.core.StyleInfo
import com.wusla.hams.core.Suggestion
import com.wusla.hams.core.io
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/** Is each setup step done? Drives onboarding and Home's checklist. */
data class Setup(
    val keyboardEnabled: Boolean = false,
    val keyboardSelected: Boolean = false,
    val micAllowed: Boolean = false,
    val modelReady: Boolean = false,
) {
    val complete get() = keyboardEnabled && micAllowed && modelReady
}

/** The app's screen state, loaded from the engine and reloaded when its data changes. */
class AppModel(private val context: Context, private val scope: CoroutineScope) {
    var settings by mutableStateOf<SettingsDoc?>(null)
        private set
    var setup by mutableStateOf(Setup())
        private set
    var recent by mutableStateOf<List<HistoryItem>>(emptyList())
        private set
    var dictionary by mutableStateOf<List<DictionaryEntry>>(emptyList())
        private set
    var suggestions by mutableStateOf<List<Suggestion>>(emptyList())
        private set
    var snippets by mutableStateOf<List<Snippet>>(emptyList())
        private set
    var models by mutableStateOf<List<ModelStatus>>(emptyList())
        private set
    var styles by mutableStateOf<List<StyleInfo>>(emptyList())
        private set
    var stats by mutableStateOf<Stats?>(null)
        private set
    /** Model id → download progress 0..1 while downloading. */
    val downloads = mutableStateMapOf<String, Float>()
    var message by mutableStateOf<String?>(null)

    fun reload() {
        if (Hams.engineOrNull == null) return
        scope.launch {
            io { SettingsDoc.parse(settingsJson()) }.onSuccess { settings = it }
            io { Parse.history(historyRecentJson(30)) }.onSuccess { recent = it }
            io { Parse.dictionary(dictionaryJson()) }.onSuccess { dictionary = it }
            io { Parse.suggestions(suggestionsJson()) }.onSuccess { suggestions = it }
            io { Parse.snippets(snippetsJson()) }.onSuccess { snippets = it }
            io { Parse.models(modelsJson()) }.onSuccess { models = it }
            io { Parse.stats(statsJson()) }.onSuccess { stats = it }
            if (styles.isEmpty()) io { Parse.styles(stylesJson()) }.onSuccess { styles = it }
            refreshSetup()
        }
    }

    fun refreshSetup() {
        val imm = context.getSystemService(InputMethodManager::class.java)
        val ours = context.packageName
        val enabled = imm?.enabledInputMethodList?.any { it.packageName == ours } == true
        val selected = Settings.Secure.getString(context.contentResolver, Settings.Secure.DEFAULT_INPUT_METHOD)
            ?.startsWith("$ours/") == true
        setup = Setup(
            keyboardEnabled = enabled,
            keyboardSelected = selected,
            micAllowed = Mic.hasPermission(context),
            modelReady = Hams.engineOrNull?.modelReady() == true,
        )
    }

    /** Save a settings change (optimistically shown, then confirmed by the engine). */
    fun save(change: SettingsDoc.() -> SettingsDoc) {
        val current = settings ?: return
        val next = current.change()
        settings = next
        scope.launch {
            io { saveSettingsJson(next.json.toString()) }
                .onFailure { message = it.message; reload() }
        }
    }

    fun setApiKey(provider: String, key: String) = run("Saved") {
        setApiKey(provider, key)
    }

    fun download(id: String) {
        if (downloads.containsKey(id)) return
        downloads[id] = 0f
        scope.launch {
            val listener = object : DownloadListener {
                override fun onProgress(fraction: Float) {
                    scope.launch { downloads[id] = fraction }
                }
            }
            io { downloadModel(id, listener) }.onFailure { message = it.message }
            downloads.remove(id)
            reload()
        }
    }

    /** Run an engine action; show its error (or `done`) as a message. */
    fun run(done: String? = null, action: com.wusla.hams.core.HamsEngine.() -> Unit) {
        scope.launch {
            io(action).onSuccess { if (done != null) message = done }.onFailure { message = it.message }
            reload()
        }
    }
}
