package com.wusla.hams.core

import org.json.JSONArray
import org.json.JSONObject

// Kotlin shapes of the engine's JSON (the same types the desktop app gets in
// src/lib/bindings.ts). Parsed with org.json, which ships with Android.

data class HistoryItem(
    val id: Long,
    val createdAt: Long,
    val text: String,
    /** What Whisper heard. */
    val raw: String,
    val app: String,
    val style: String,
    val delivery: String,
    val durationMs: Long,
    val words: Long,
    val pinned: Boolean,
)

data class DictionaryEntry(val id: Long, val term: String, val replacement: String?, val source: String)
data class Snippet(val id: Long, val trigger: String, val expansion: String)
data class Suggestion(val id: Long, val heard: String, val correct: String, val count: Long)

data class ModelStatus(
    val id: String,
    val label: String,
    val sizeMb: Int,
    val multilingual: Boolean,
    val note: String,
    val installed: Boolean,
)

data class StyleInfo(val id: String, val label: String, val prompt: String, val polish: Boolean)

data class Restyled(val text: String, val polished: Boolean)

enum class RunKind { KEEP, DROP, ADD }
data class Run(val kind: RunKind, val words: List<String>)
data class WordDiff(val heard: List<Run>, val written: List<Run>, val dropped: Int, val added: Int)

data class Stats(
    val totalWords: Long,
    val totalDictations: Long,
    val wordsToday: Long,
    val minutesSaved: Long,
    val currentStreak: Long,
    /** Local date "YYYY-MM-DD" → words, for days with activity (last 120). */
    val activity: Map<String, Long> = emptyMap(),
)

private inline fun <T> JSONArray.map(transform: (JSONObject) -> T): List<T> =
    (0 until length()).map { transform(getJSONObject(it)) }

private fun JSONObject.optStringOrNull(name: String): String? = if (isNull(name)) null else optString(name)

object Parse {
    fun history(json: String): List<HistoryItem> = JSONArray(json).map {
        HistoryItem(
            id = it.getLong("id"), createdAt = it.getLong("created_at"), text = it.getString("text"),
            raw = it.optString("raw", it.getString("text")), app = it.optString("app"), style = it.optString("style"),
            delivery = it.optString("delivery"), durationMs = it.optLong("duration_ms"), words = it.optLong("words"),
            pinned = it.optBoolean("pinned"),
        )
    }

    fun dictionary(json: String): List<DictionaryEntry> = JSONArray(json).map {
        DictionaryEntry(it.getLong("id"), it.getString("term"), it.optStringOrNull("replacement"), it.optString("source"))
    }

    fun snippets(json: String): List<Snippet> = JSONArray(json).map {
        Snippet(it.getLong("id"), it.getString("trigger"), it.getString("expansion"))
    }

    fun suggestions(json: String): List<Suggestion> = JSONArray(json).map {
        Suggestion(it.getLong("id"), it.getString("heard"), it.getString("correct"), it.optLong("count"))
    }

    fun models(json: String): List<ModelStatus> = JSONArray(json).map {
        ModelStatus(
            it.getString("id"), it.getString("label"), it.optInt("size_mb"), it.optBoolean("multilingual"),
            it.optString("note"), it.optBoolean("installed"),
        )
    }

    fun styles(json: String): List<StyleInfo> = JSONArray(json).map {
        StyleInfo(it.getString("id"), it.getString("label"), it.optString("prompt"), it.optBoolean("polish"))
    }

    fun restyled(json: String): Restyled = JSONObject(json).let { Restyled(it.getString("text"), it.optBoolean("polished")) }

    fun stats(json: String): Stats = JSONObject(json).let {
        val days = it.optJSONArray("activity")
        val activity = buildMap {
            if (days != null) for (i in 0 until days.length()) {
                val day = days.optJSONArray(i) ?: continue
                put(day.optString(0), day.optLong(1))
            }
        }
        Stats(
            it.optLong("total_words"), it.optLong("total_dictations"), it.optLong("words_today"),
            it.optLong("minutes_saved"), it.optLong("current_streak"), activity,
        )
    }

    fun diff(json: String): WordDiff {
        val o = JSONObject(json)
        fun runs(name: String) = o.optJSONArray(name)?.map { r ->
            val words = r.getJSONArray("words")
            Run(RunKind.valueOf(r.getString("kind").uppercase()), (0 until words.length()).map { words.getString(it) })
        } ?: emptyList()
        return WordDiff(runs("heard"), runs("written"), o.optInt("dropped"), o.optInt("added"))
    }
}

/**
 * The settings object, kept as JSON so fields this app doesn't show (desktop
 * shortcuts, style overrides) survive a save untouched.
 */
class SettingsDoc(val json: JSONObject, val hasApiKey: Boolean) {
    val model get() = json.optString("model")
    val language get() = json.optString("language", "auto")
    val languages: List<String>
        get() = json.optJSONArray("languages")?.let { a -> (0 until a.length()).map { a.getString(it) } } ?: emptyList()
    val removeFillers get() = json.optBoolean("remove_fillers", true)
    val voiceCommands get() = json.optBoolean("voice_commands", true)
    val livePreview get() = json.optBoolean("live_preview", false)
    val appStyles get() = json.optBoolean("app_styles", true)
    val learnFromEdits get() = json.optBoolean("learn_from_edits", true)
    val learnFromFields get() = json.optBoolean("learn_from_fields", false)
    val doubleTapLock get() = json.optBoolean("double_tap_lock", true)
    val onboardingDone get() = json.optBoolean("onboarding_done", false)
    val ai: JSONObject get() = json.optJSONObject("ai") ?: JSONObject()
    val aiProvider get() = ai.optString("provider", "off")
    val aiModel get() = ai.optString("model")
    val aiBaseUrl get() = ai.optString("base_url")
    val smartSkipWords get() = ai.optInt("smart_skip_words", 4)

    /** A copy with `change` applied to the JSON. */
    fun edit(change: JSONObject.() -> Unit): SettingsDoc =
        SettingsDoc(JSONObject(json.toString()).apply(change), hasApiKey)

    fun editAi(change: JSONObject.() -> Unit): SettingsDoc = edit {
        put("ai", (optJSONObject("ai") ?: JSONObject()).apply(change))
    }

    companion object {
        fun parse(view: String): SettingsDoc {
            val o = JSONObject(view)
            return SettingsDoc(o.getJSONObject("settings"), o.optBoolean("has_api_key"))
        }
    }
}

/** AI providers the engine knows, as (id, label, needs a key). */
val AI_PROVIDERS = listOf(
    Triple("off", "Off (local clean-up only)", false),
    Triple("gemini", "Google Gemini", true),
    Triple("anthropic", "Anthropic Claude", true),
    Triple("open_ai", "OpenAI", true),
    Triple("groq", "Groq", true),
    Triple("open_router", "OpenRouter", true),
    Triple("ollama", "Ollama (your own server)", false),
    Triple("custom", "Other OpenAI-compatible", true),
)

/** Whisper languages people pick most, as (code, name). "auto" detects. */
val LANGUAGES = listOf(
    "auto" to "Detect automatically", "en" to "English", "ml" to "Malayalam", "hi" to "Hindi",
    "ta" to "Tamil", "ar" to "Arabic", "ur" to "Urdu", "bn" to "Bengali", "te" to "Telugu", "kn" to "Kannada",
    "mr" to "Marathi", "es" to "Spanish", "fr" to "French", "de" to "German", "pt" to "Portuguese",
    "id" to "Indonesian", "ms" to "Malay", "tr" to "Turkish", "ru" to "Russian", "zh" to "Chinese", "ja" to "Japanese",
)
