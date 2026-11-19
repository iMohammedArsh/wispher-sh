package com.wusla.hams.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.wusla.hams.R
import com.wusla.hams.core.Lang
import com.wusla.hams.core.POPULAR_LANGUAGES
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.components.Chip
import com.wusla.hams.ui.components.Field
import com.wusla.hams.ui.components.HamsIcon
import com.wusla.hams.ui.components.LinkButton
import com.wusla.hams.ui.components.Muted
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.Pill
import com.wusla.hams.ui.components.PrimaryButton
import com.wusla.hams.ui.components.RoundIcon
import com.wusla.hams.ui.components.Rule
import com.wusla.hams.ui.components.ScreenTitle
import com.wusla.hams.ui.components.SectionTitle
import com.wusla.hams.ui.components.Tone
import com.wusla.hams.ui.components.ValueRow
import com.wusla.hams.ui.components.bounceClick
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens

/** The settings limit (Settings::validate). */
private const val MAX_LANGUAGES = 12

/**
 * Languages: the ones you speak (main first), the speech pack that suits them,
 * and whether to detect or stick to one. Whisper has no per-language download:
 * the speech pack decides how well each language works.
 */
@Composable
fun LanguagesArea(model: AppModel, openPacks: () -> Unit) {
    val c = Tokens.colors
    val s = model.settings ?: return
    val mine = s.languages
    var adding by remember { mutableStateOf(false) }

    ScreenTitle("Languages", "Add the languages you speak. Hams writes in the one you use, and gets the speech pack that suits them.")

    Panel(padding = PaddingValues(horizontal = 16.dp, vertical = 6.dp)) {
        if (mine.isEmpty()) {
            Muted(
                "None yet, so Hams guesses from any language. That's slower and can pick the wrong one.",
                Modifier.padding(vertical = 12.dp),
            )
            Rule()
        }
        mine.forEachIndexed { i, code ->
            LanguageRow(
                lang = model.lang(code) ?: Lang(code, code, code),
                main = i == 0,
                onMain = { model.saveLanguages(listOf(code) + (mine - code)) },
                onRemove = { model.saveLanguages(mine - code) },
            )
            Rule()
        }
        Row(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp))
                .bounceClick(enabled = mine.size < MAX_LANGUAGES) { adding = true }
                .padding(vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(Modifier.size(40.dp).clip(RoundedCornerShape(12.dp)).background(c.active), contentAlignment = Alignment.Center) {
                Text("+", style = HamsType.heading, color = if (c.dark) c.mint else c.primary)
            }
            Spacer(Modifier.width(14.dp))
            Text(
                if (mine.size < MAX_LANGUAGES) "Add a language" else "That's the most Hams can listen for",
                style = HamsType.written.copy(fontWeight = FontWeight.Medium),
                color = if (mine.size < MAX_LANGUAGES) c.text else c.faint,
            )
        }
    }

    SectionTitle("Speech pack")
    PackCard(model, mine, openPacks)

    if (mine.size >= 2) {
        SectionTitle("When you speak")
        Panel {
            Muted("Detecting picks among your languages each time. Sticking to one is faster. You can also switch with the language button on the keyboard.")
            Spacer(Modifier.height(12.dp))
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Chip("Detect", { model.useLanguage("auto") }, selected = s.language == "auto" || s.language !in mine)
                mine.forEach { code ->
                    Chip(model.lang(code)?.native ?: code, { model.useLanguage(code) }, selected = s.language == code)
                }
            }
        }
    }

    if (adding) {
        AddLanguageSheet(
            model, chosen = mine,
            onPick = { model.saveLanguages(mine + it); adding = false },
            onDismiss = { adding = false },
        )
    }
}

@Composable
private fun LanguageRow(lang: Lang, main: Boolean, onMain: () -> Unit, onRemove: () -> Unit) {
    val c = Tokens.colors
    Row(Modifier.fillMaxWidth().padding(vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(lang.title, style = HamsType.section.copy(fontWeight = FontWeight.Medium), color = c.text)
            lang.subtitle?.let { Text(it, style = HamsType.small, color = c.muted) }
        }
        if (main) Pill("Main", Tone.Positive) else LinkButton("Make main", onMain, color = c.muted)
        Spacer(Modifier.width(6.dp))
        RoundIcon(R.drawable.ic_close, onRemove, description = "Remove ${lang.name}", size = 36.dp)
    }
}

/** "English", "English and Malayalam", "English, Hindi and Malayalam". */
private fun joinNames(names: List<String>): String = when (names.size) {
    0 -> ""
    1 -> names[0]
    else -> names.dropLast(1).joinToString(", ") + " and " + names.last()
}

/** The pack that suits your languages: download it, switch to it, or see that it's in use. */
@Composable
private fun PackCard(model: AppModel, mine: List<String>, openPacks: () -> Unit) {
    val c = Tokens.colors
    val s = model.settings ?: return
    val recommended = model.models.firstOrNull { it.recommended } ?: return
    val current = model.models.firstOrNull { it.id == s.model }
    val englishOnly = mine.all { it == "en" }
    // Already on something at least as good (a bigger pack that covers these languages)?
    val covered = current != null && current.installed &&
        current.sizeMb >= recommended.sizeMb && (current.multilingual || englishOnly)
    val shown = if (covered) current!! else recommended
    val progress = model.downloads[shown.id]
    val names = joinNames(mine.mapNotNull { model.lang(it)?.name })

    Panel {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(shown.label, style = HamsType.heading, color = c.text)
            Spacer(Modifier.width(8.dp))
            Text("${shown.sizeMb} MB", style = HamsType.small, color = c.muted)
            Spacer(Modifier.weight(1f))
            when {
                shown.id == s.model && shown.installed -> Pill("In use", Tone.Positive)
                shown.installed -> Pill("Downloaded")
            }
        }
        Spacer(Modifier.height(6.dp))
        Muted(
            when {
                covered && shown.id != recommended.id -> "Covers ${names.ifEmpty { "your languages" }}, and it's already on this phone."
                !shown.multilingual -> "Made for English only: the same size as the all-language pack, and more accurate for English."
                names.isEmpty() -> shown.note
                else -> "Best for $names on a phone. ${shown.note}"
            },
        )
        Spacer(Modifier.height(14.dp))
        when {
            progress != null -> {
                LinearProgressIndicator(
                    progress = { progress }, modifier = Modifier.fillMaxWidth().height(6.dp),
                    color = c.data, trackColor = c.active, strokeCap = StrokeCap.Round, gapSize = 0.dp, drawStopIndicator = {},
                )
                Spacer(Modifier.height(6.dp))
                Muted("Downloading… ${(progress * 100).toInt()}%. You can keep using Hams.")
            }
            !shown.installed -> {
                model.downloadFailures[shown.id]?.let { why ->
                    Text("The download stopped: $why", style = HamsType.small, color = c.negative)
                    Muted("Try again and Hams carries on from where it stopped.")
                    Spacer(Modifier.height(10.dp))
                }
                PrimaryButton(
                    (if (model.downloadFailures.containsKey(shown.id)) "Try again" else "Download") + " · ${shown.sizeMb} MB",
                    { model.download(shown.id, thenUse = true) }, Modifier.fillMaxWidth(),
                )
            }
            shown.id != s.model -> PrimaryButton("Use ${shown.label}", { model.save { edit { put("model", shown.id) } } }, Modifier.fillMaxWidth())
            else -> Muted("Ready. Dictation runs on this phone, even offline.")
        }
        Spacer(Modifier.height(4.dp))
        ValueRow("All speech packs", "", openPacks)
    }
}

/** Full-screen search over every language Whisper can write. Picking one adds it. */
@Composable
fun AddLanguageSheet(model: AppModel, chosen: List<String>, onPick: (String) -> Unit, onDismiss: () -> Unit) {
    val c = Tokens.colors
    var query by remember { mutableStateOf("") }
    val all = remember(model.languages) { model.languages.sortedBy { it.name } }
    val suggested = remember(model.languages) {
        (model.deviceLanguages() + POPULAR_LANGUAGES).distinct().mapNotNull { model.lang(it) }
    }
    val q = query.trim()
    val matches = if (q.isEmpty()) emptyList() else all.filter {
        it.name.contains(q, ignoreCase = true) || it.native.contains(q, ignoreCase = true) || it.code.equals(q, ignoreCase = true)
    }

    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        Column(
            Modifier.fillMaxSize().background(c.ground)
                .windowInsetsPadding(WindowInsets.safeDrawing).imePadding()
                .padding(horizontal = 20.dp),
        ) {
            Row(Modifier.fillMaxWidth().padding(top = 8.dp, bottom = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                RoundIcon(R.drawable.ic_back, onDismiss, description = "Back")
                Spacer(Modifier.width(12.dp))
                Text("Add a language", style = HamsType.heading, color = c.text)
            }
            Field(query, { query = it }, "", placeholder = "Search: Malayalam, മലയാളം, Hindi…")
            Spacer(Modifier.height(8.dp))
            LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = 24.dp)) {
                if (q.isEmpty()) {
                    item { SectionTitle("Suggested") }
                    items(suggested, key = { "s-" + it.code }) { PickRow(it, it.code in chosen) { onPick(it.code) } }
                    item { SectionTitle("All languages") }
                    items(all, key = { "a-" + it.code }) { PickRow(it, it.code in chosen) { onPick(it.code) } }
                } else if (matches.isEmpty()) {
                    item { Muted("Whisper doesn't know \"$q\". Try its English name.", Modifier.padding(top = 20.dp)) }
                } else {
                    items(matches, key = { it.code }) { PickRow(it, it.code in chosen) { onPick(it.code) } }
                }
            }
        }
    }
}

@Composable
private fun PickRow(lang: Lang, chosen: Boolean, onPick: () -> Unit) {
    val c = Tokens.colors
    Row(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp))
            .bounceClick(enabled = !chosen, onClick = onPick)
            .padding(horizontal = 4.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(lang.title, style = HamsType.written.copy(fontWeight = FontWeight.Medium), color = c.text)
            lang.subtitle?.let { Text(it, style = HamsType.small, color = c.muted) }
        }
        if (chosen) HamsIcon(R.drawable.ic_check, tint = c.data, size = 20.dp)
    }
}
