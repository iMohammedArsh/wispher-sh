package com.wusla.hams.ui

import android.content.Context
import android.net.ConnectivityManager
import android.os.StatFs
import android.provider.Settings
import android.view.inputmethod.InputMethodManager
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.wusla.hams.core.DictionaryEntry
import com.wusla.hams.core.Downloads
import com.wusla.hams.core.Hams
import com.wusla.hams.core.HistoryItem
import com.wusla.hams.core.Lang
import com.wusla.hams.core.Mic
import com.wusla.hams.core.ModelStatus
import com.wusla.hams.core.Parse
import com.wusla.hams.core.SettingsDoc
import com.wusla.hams.core.Stats
import com.wusla.hams.core.Snippet
import com.wusla.hams.core.StyleInfo
import com.wusla.hams.core.Suggestion
import com.wusla.hams.core.io
import com.wusla.hams.core.whisperCodes
import org.json.JSONArray
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/** Is each setup step done? Drives onboarding and Home's checklist. */
data class Setup(
    val keyboardEnabled: Boolean = false,
    val keyboardSelected: Boolean = false,
    val micAllowed: Boolean = false,
    val modelReady: Boolean = false,
) {
    /** Everything Hams needs, including being the keyboard that is in use. */
    val complete get() = keyboardEnabled && keyboardSelected && micAllowed && modelReady

    /** How many of the four things are still to do. */
    val missing get() = listOf(keyboardEnabled, keyboardSelected, micAllowed, modelReady).count { !it }
}

/** A download that waits for the user: on a metered network, or without room for it. */
data class PendingDownload(val id: String, val label: String, val sizeMb: Int, val thenUse: Boolean, val check: DownloadCheck)

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
    /** Every language Whisper can write (loaded once). */
    var languages by mutableStateOf<List<Lang>>(emptyList())
        private set
    /** Pack id → download progress 0..1 while downloading. Owned by the process, not this screen state. */
    val downloads: Map<String, Float> get() = Downloads.progress

    /** Pack id → why its last download failed; stays until it is tried again. */
    val downloadFailures: Map<String, String> get() = Downloads.failures

    /** A download waiting for the user's answer (see [download]). */
    var pendingDownload by mutableStateOf<PendingDownload?>(null)
        private set
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
            if (languages.isEmpty()) io { Parse.languages(languagesJson()) }.onSuccess { languages = it }
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

    fun lang(code: String): Lang? = languages.firstOrNull { it.code == code }

    /** The phone's own languages (Android settings), as Whisper codes. */
    fun deviceLanguages(): List<String> {
        val tags = android.os.LocaleList.getDefault().let { list -> (0 until list.size()).map { list[it].toLanguageTag() } }
        return whisperCodes(tags, languages.map { it.code }.toSet())
    }

    /**
     * Save the languages the user speaks, main first. The engine fixes the
     * language when there's one and picks the pack that suits them if none is
     * downloaded yet; shown at once, then confirmed by a reload.
     */
    fun saveLanguages(codes: List<String>) {
        settings = settings?.edit {
            put("languages", JSONArray(codes))
            if (codes.size == 1) put("language", codes[0]) else if (optString("language") !in codes) put("language", "auto")
        }
        run { this.setLanguages(codes) }
    }

    /** Dictate in one language, or "auto" to detect among the user's languages. */
    fun useLanguage(code: String) {
        settings = settings?.edit { put("language", code) }
        run { this.setLanguage(code) }
    }

    fun setApiKey(provider: String, key: String) = run("Saved") {
        setApiKey(provider, key)
    }

    /**
     * Download a speech pack; with `thenUse`, switch to it once it's on the
     * phone. On mobile data it asks first, and with too little free space it
     * says so instead of failing half way (see [pendingDownload]).
     */
    fun download(id: String, thenUse: Boolean = false) {
        if (downloads.containsKey(id)) return
        val info = models.firstOrNull { it.id == id }
        val check = downloadCheck(info?.sizeMb ?: 0, freeBytes(), onMeteredNetwork())
        if (check == DownloadCheck.Ok) {
            Downloads.start(id, thenUse)
        } else {
            pendingDownload = PendingDownload(id, info?.label ?: id, info?.sizeMb ?: 0, thenUse, check)
        }
    }

    /** "Download anyway" on mobile data. Does nothing for a download that was refused for space. */
    fun confirmDownload() {
        pendingDownload?.takeIf { it.check == DownloadCheck.MobileData }?.let { Downloads.start(it.id, it.thenUse) }
        pendingDownload = null
    }

    fun dismissDownload() {
        pendingDownload = null
    }

    /** Free space where the models live (the app's own storage). */
    private fun freeBytes(): Long =
        try { StatFs(context.filesDir.path).availableBytes } catch (_: Throwable) { Long.MAX_VALUE }

    /** Mobile data or a hotspot; unknown counts as not metered, so a failed lookup never blocks a download. */
    private fun onMeteredNetwork(): Boolean =
        try { context.getSystemService(ConnectivityManager::class.java)?.isActiveNetworkMetered == true } catch (_: Throwable) { false }

    /** Run an engine action; show its error (or `done`) as a message. */
    fun run(done: String? = null, action: com.wusla.hams.core.HamsEngine.() -> Unit) {
        scope.launch {
            io(action).onSuccess { if (done != null) message = done }.onFailure { message = it.message }
            reload()
        }
    }
}
