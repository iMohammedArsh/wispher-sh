package com.wusla.hams.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.wusla.hams.core.AI_PROVIDERS
import com.wusla.hams.core.LANGUAGES
import com.wusla.hams.core.Parse
import com.wusla.hams.core.io
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.components.BackBar
import com.wusla.hams.ui.components.Chip
import com.wusla.hams.ui.components.ConfirmLink
import com.wusla.hams.ui.components.Field
import com.wusla.hams.ui.components.HeardWritten
import com.wusla.hams.ui.components.LinkButton
import com.wusla.hams.ui.components.Muted
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.Pill
import com.wusla.hams.ui.components.PrimaryButton
import com.wusla.hams.ui.components.QuietButton
import com.wusla.hams.ui.components.Rule
import com.wusla.hams.ui.components.ScreenTitle
import com.wusla.hams.ui.components.SectionTitle
import com.wusla.hams.ui.components.SwitchRow
import com.wusla.hams.ui.components.Tone
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens
import kotlinx.coroutines.launch
import org.json.JSONArray

/** You and its settings areas, one at a time: pick an area, change it, go back. */
@Composable
fun SettingsScreen(model: AppModel, area: String?, requestMic: ((Boolean) -> Unit) -> Unit, setArea: (String?) -> Unit) {
    Column(
        Modifier
            .fillMaxSize()
            .imePadding()
            .verticalScroll(rememberScrollState())
            .padding(start = 20.dp, end = 20.dp, top = 12.dp, bottom = 120.dp),
    ) {
        when (area) {
            null -> YouIndex(model, setArea)
            else -> {
                BackBar({ setArea(null) }, "You")
                Spacer(Modifier.height(8.dp))
                when (area) {
                    "model" -> ModelArea(model)
                    "speech" -> SpeechArea(model)
                    "ai" -> AiArea(model)
                    "styles" -> StylesArea(model)
                    "privacy" -> PrivacyArea(model)
                }
            }
        }
    }
}

@Composable
private fun ModelArea(model: AppModel) {
    val c = Tokens.colors
    val s = model.settings ?: return
    ScreenTitle("Speech model", "Whisper runs on this phone. Speech never leaves it for transcription.")
    model.models.forEach { m ->
        val selected = m.id == s.model
        val progress = model.downloads[m.id]
        Panel(modifier = Modifier.padding(bottom = 12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(m.label, style = HamsType.section, color = c.text)
                Spacer(Modifier.width(8.dp))
                Text("${m.sizeMb} MB", style = HamsType.small, color = c.muted)
                Spacer(Modifier.weight(1f))
                when {
                    selected && m.installed -> Pill("In use", Tone.Positive)
                    selected -> Pill("Selected", Tone.Warning)
                    m.installed -> Pill("Downloaded")
                }
            }
            Spacer(Modifier.height(4.dp))
            Muted(m.note)
            Spacer(Modifier.height(10.dp))
            if (progress != null) {
                LinearProgressIndicator(progress = { progress }, modifier = Modifier.fillMaxWidth().height(6.dp), color = c.data, trackColor = c.active, strokeCap = androidx.compose.ui.graphics.StrokeCap.Round, gapSize = 0.dp, drawStopIndicator = {})
                Spacer(Modifier.height(4.dp))
                Muted("Downloading… ${(progress * 100).toInt()}%")
            } else {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    if (!m.installed) PrimaryButton("Download", { model.download(m.id) })
                    if (!selected) QuietButton("Use this", { model.save { edit { put("model", m.id) } } })
                    if (m.installed && !selected) ConfirmLink("Delete", "Delete?") { model.run { deleteModel(m.id) } }
                }
            }
        }
    }
}

@Composable
private fun SpeechArea(model: AppModel) {
    val c = Tokens.colors
    val s = model.settings ?: return
    var menu by remember { mutableStateOf(false) }
    ScreenTitle("Language and clean-up")
    Panel {
        Text("Language", style = HamsType.bodyStrong, color = c.text)
        Spacer(Modifier.height(4.dp))
        Muted("Whisper writes in the language it detects. If it sometimes picks the wrong one, choose yours here or limit detection below.")
        Spacer(Modifier.height(8.dp))
        Column {
            QuietButton(LANGUAGES.firstOrNull { it.first == s.language }?.second ?: s.language, { menu = true })
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                LANGUAGES.forEach { (code, name) ->
                    DropdownMenuItem(text = { Text(name) }, onClick = {
                        menu = false
                        model.save { edit { put("language", code) } }
                    })
                }
            }
        }
        if (s.language == "auto") {
            Spacer(Modifier.height(16.dp))
            Text("Languages you speak", style = HamsType.bodyStrong, color = c.text)
            Muted("Detection picks only from these. The first is used when it's unsure.")
            Spacer(Modifier.height(8.dp))
            val chosen = s.languages
            androidx.compose.foundation.layout.FlowRow(
                horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                LANGUAGES.drop(1).forEach { (code, name) ->
                    val on = code in chosen
                    Chip(if (on && chosen.firstOrNull() == code) "$name · main" else name, {
                        val next = if (on) chosen - code else chosen + code
                        model.save { edit { put("languages", JSONArray(next)) } }
                    }, selected = on)
                }
            }
        }
    }
    Spacer(Modifier.height(12.dp))
    Panel {
        SwitchRow("Remove filler words", "um, uh, you know, repeated words", s.removeFillers, { v -> model.save { edit { put("remove_fillers", v) } } })
        Rule()
        SwitchRow("Voice commands", "\"new line\", \"new paragraph\", \"scratch that\"", s.voiceCommands, { v -> model.save { edit { put("voice_commands", v) } } })
        Rule()
        SwitchRow("Live preview", "Show words while you speak (uses more battery)", s.livePreview, { v -> model.save { edit { put("live_preview", v) } } })
        Rule()
        SwitchRow("Double-tap for hands-free", "Tap the mic twice to keep recording without holding", s.doubleTapLock, { v -> model.save { edit { put("double_tap_lock", v) } } })
        Rule()
        SwitchRow("Learn from my edits", "Suggest dictionary fixes when you correct a dictation in History", s.learnFromEdits, { v -> model.save { edit { put("learn_from_edits", v) } } })
        Rule()
        SwitchRow("Learn from fixes in the app", "Re-read the field for a minute after inserting and suggest fixes for words you correct there", s.learnFromFields, { v -> model.save { edit { put("learn_from_fields", v) } } })
    }
}

@Composable
private fun AiArea(model: AppModel) {
    val c = Tokens.colors
    val s = model.settings ?: return
    val scope = rememberCoroutineScope()
    var menu by remember { mutableStateOf(false) }
    var key by remember { mutableStateOf("") }
    var testResult by remember { mutableStateOf<String?>(null) }
    var testing by remember { mutableStateOf(false) }
    var aiModel by remember(s.aiProvider) { mutableStateOf(s.aiModel) }
    var baseUrl by remember(s.aiProvider) { mutableStateOf(s.aiBaseUrl) }
    var defaultModel by remember { mutableStateOf("") }
    val provider = AI_PROVIDERS.firstOrNull { it.first == s.aiProvider } ?: AI_PROVIDERS.first()

    LaunchedEffect(s.aiProvider) {
        io { defaultAiModel(s.aiProvider) }.onSuccess { defaultModel = it }
    }

    ScreenTitle("AI polish", "Off by default. When on, the cleaned-up text (never audio) is sent to the provider you choose, with your own key.")
    Panel {
        Text("Provider", style = HamsType.bodyStrong, color = c.text)
        Spacer(Modifier.height(8.dp))
        Column {
            QuietButton(provider.second, { menu = true })
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                AI_PROVIDERS.forEach { (id, label, _) ->
                    DropdownMenuItem(text = { Text(label) }, onClick = {
                        menu = false
                        model.save { editAi { put("provider", id); put("model", "") } }
                    })
                }
            }
        }
        if (provider.first != "off") {
            if (provider.third) {
                Spacer(Modifier.height(16.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("API key", style = HamsType.bodyStrong, color = c.text, modifier = Modifier.weight(1f))
                    if (s.hasApiKey) Pill("Saved", Tone.Positive) else Pill("Needed", Tone.Warning)
                }
                Spacer(Modifier.height(6.dp))
                Field(key, { key = it }, if (s.hasApiKey) "Replace key" else "Paste your key", secret = true)
                Spacer(Modifier.height(8.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    PrimaryButton("Save key", { model.setApiKey(provider.first, key); key = "" }, enabled = key.isNotBlank())
                    if (s.hasApiKey) ConfirmLink("Remove key", "Remove?") { model.setApiKey(provider.first, "") }
                }
                Spacer(Modifier.height(4.dp))
                Muted("Stored encrypted with a key held by Android's secure hardware.")
            }
            if (provider.first == "custom" || provider.first == "ollama") {
                Spacer(Modifier.height(12.dp))
                Field(baseUrl, { baseUrl = it }, "Server URL", placeholder = "http://192.168.1.10:11434/v1")
            }
            Spacer(Modifier.height(12.dp))
            Field(aiModel, { aiModel = it }, "Model", placeholder = defaultModel.ifEmpty { "Recommended" })
            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                QuietButton("Save", { model.save { editAi { put("model", aiModel.trim()); put("base_url", baseUrl.trim()) } } })
                QuietButton(if (testing) "Testing…" else "Test", {
                    testing = true
                    testResult = null
                    scope.launch {
                        testResult = io { testAi() }.fold({ "Works: $it" }, { it.message ?: "Failed" })
                        testing = false
                    }
                }, enabled = !testing)
            }
            testResult?.let {
                Spacer(Modifier.height(8.dp))
                Muted(it)
            }
        }
    }
    if (provider.first != "off") {
        Spacer(Modifier.height(12.dp))
        Panel {
            Text("Skip AI for short dictations", style = HamsType.bodyStrong, color = c.text)
            Muted("Dictations under ${s.smartSkipWords} words are only cleaned up locally: instant, and nothing is sent.")
            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                listOf(0, 4, 8, 15).forEach { n ->
                    Chip(if (n == 0) "Never" else "$n words", { model.save { editAi { put("smart_skip_words", n) } } }, selected = s.smartSkipWords == n)
                }
            }
        }
    }
}

@Composable
private fun StylesArea(model: AppModel) {
    val c = Tokens.colors
    val s = model.settings ?: return
    val scope = rememberCoroutineScope()
    var sample by remember { mutableStateOf("um so I think we should uh move the meeting to thursday if that works for everyone") }
    var chosen by remember { mutableStateOf("messaging") }
    var result by remember { mutableStateOf<String?>(null) }
    var trying by remember { mutableStateOf(false) }

    ScreenTitle("Styles", "Hams writes differently for chats, email and documents.")
    Panel {
        SwitchRow("Match the app", "WhatsApp gets a chat style, Gmail an email style, and so on", s.appStyles, { v -> model.save { edit { put("app_styles", v) } } })
    }
    Spacer(Modifier.height(12.dp))
    Panel {
        model.styles.forEachIndexed { i, style ->
            if (i > 0) Rule()
            Column(Modifier.padding(vertical = 10.dp)) {
                Text(style.label, style = HamsType.bodyStrong, color = c.text)
                Text(style.prompt, style = HamsType.small, color = c.muted)
            }
        }
    }
    SectionTitle("Try it")
    Panel {
        Field(sample, { sample = it }, "Spoken text", singleLine = false)
        Spacer(Modifier.height(8.dp))
        androidx.compose.foundation.layout.FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            model.styles.forEach { style -> Chip(style.label, { chosen = style.id; result = null }, selected = chosen == style.id) }
        }
        Spacer(Modifier.height(10.dp))
        QuietButton(if (trying) "Writing…" else "Write it", {
            trying = true
            scope.launch {
                result = io { Parse.restyled(styleTry(chosen, sample)).text }.getOrElse { it.message }
                trying = false
            }
        }, enabled = !trying && sample.isNotBlank())
        result?.let {
            Spacer(Modifier.height(12.dp))
            HeardWritten(sample, it, summary = true)
        }
    }
}

@Composable
private fun PrivacyArea(model: AppModel) {
    ScreenTitle("Privacy")
    Panel {
        Muted("Your voice is transcribed on this phone by Whisper. Audio is kept in memory only while you dictate and is never saved or sent.")
        Spacer(Modifier.height(10.dp))
        Muted("History, your dictionary and snippets are stored only on this phone and excluded from backups.")
        Spacer(Modifier.height(10.dp))
        Muted("AI polish is off unless you turn it on. Then only the cleaned-up text goes to the provider you picked, using your key.")
        Spacer(Modifier.height(10.dp))
        Muted("The keyboard reads the few characters before the cursor to space words correctly, and never works in password fields.")
    }
    Spacer(Modifier.height(12.dp))
    Panel {
        Muted("Full policy: hams.wusla.co/privacy")
    }
}
