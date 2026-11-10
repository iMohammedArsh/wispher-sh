package com.wusla.hams.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.components.BackBar
import com.wusla.hams.ui.components.Chip
import com.wusla.hams.ui.components.ConfirmLink
import com.wusla.hams.ui.components.Field
import com.wusla.hams.ui.components.LinkButton
import com.wusla.hams.ui.components.Muted
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.PrimaryButton
import com.wusla.hams.ui.components.Rule
import com.wusla.hams.ui.components.ScreenTitle
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens

/**
 * Words Hams should get right: names and terms (sent to Whisper and the AI),
 * fixes for words it keeps mishearing, corrections it learned and wants
 * approved, and snippets (say the trigger, get the text).
 */
@Composable
fun WordsScreen(model: AppModel, onBack: () -> Unit) {
    val c = Tokens.colors
    var section by rememberSaveable { mutableStateOf(if (model.suggestions.isNotEmpty()) 1 else 0) }
    var term by rememberSaveable { mutableStateOf("") }
    var replacement by rememberSaveable { mutableStateOf("") }
    var trigger by rememberSaveable { mutableStateOf("") }
    var expansion by rememberSaveable { mutableStateOf("") }

    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 12.dp, bottom = 120.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        item {
            BackBar(onBack, "You")
            Spacer(Modifier.height(8.dp))
            ScreenTitle("Your words", "Names, terms and fixes Hams applies to every dictation.")
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Chip("Dictionary", { section = 0 }, selected = section == 0)
                Chip(
                    if (model.suggestions.isEmpty()) "Suggestions" else "Suggestions ${model.suggestions.size}",
                    { section = 1 }, selected = section == 1,
                )
                Chip("Snippets", { section = 2 }, selected = section == 2)
            }
            Spacer(Modifier.height(8.dp))
        }

        when (section) {
            0 -> {
                item {
                    Panel {
                        Field(term, { term = it }, "Word or name", placeholder = "Tauri")
                        Spacer(Modifier.height(8.dp))
                        Field(replacement, { replacement = it }, "Written as (optional)", placeholder = "Only if Whisper hears it differently")
                        Spacer(Modifier.height(10.dp))
                        PrimaryButton("Add", {
                            val t = term.trim()
                            val r = replacement.trim()
                            // "tory" → "Tauri": the heard form is the term, the written form the replacement.
                            if (t.isNotEmpty()) model.run { dictionaryUpsert(t, r.ifEmpty { null }) }
                            term = ""
                            replacement = ""
                        }, enabled = term.isNotBlank())
                    }
                }
                if (model.dictionary.isEmpty()) item { Muted("No words yet. Add names you use often, or words Hams mishears.") }
                items(model.dictionary, key = { "d${it.id}" }) { entry ->
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(vertical = 4.dp)) {
                        Column(Modifier.weight(1f)) {
                            if (entry.replacement != null) {
                                Text("${entry.term} → ${entry.replacement}", style = HamsType.bodyStrong, color = c.text)
                                Text("Heard as \"${entry.term}\", written as \"${entry.replacement}\"", style = HamsType.small, color = c.muted)
                            } else {
                                Text(entry.term, style = HamsType.bodyStrong, color = c.text)
                                if (entry.source != "user") Text("Learned", style = HamsType.small, color = c.muted)
                            }
                        }
                        ConfirmLink("Remove", "Remove?") { model.run { dictionaryDelete(entry.id) } }
                    }
                    Rule()
                }
            }
            1 -> {
                item { Muted("When you fix a word Hams wrote, it suggests remembering the fix. Nothing changes until you approve.") }
                if (model.suggestions.isEmpty()) item { Muted("No suggestions waiting.") }
                items(model.suggestions, key = { "s${it.id}" }) { s ->
                    Panel(padding = PaddingValues(14.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Column(Modifier.weight(1f)) {
                                Text("${s.heard} → ${s.correct}", style = HamsType.bodyStrong, color = c.text)
                                if (s.count > 1) Text("Seen ${s.count} times", style = HamsType.small, color = c.muted)
                            }
                            LinkButton("Dismiss", { model.run { suggestionResolve(s.id, false) } }, color = c.muted)
                            Spacer(Modifier.width(4.dp))
                            LinkButton("Approve", { model.run("Added to Dictionary") { suggestionResolve(s.id, true) } }, color = c.data)
                        }
                    }
                }
            }
            else -> {
                item {
                    Panel {
                        Field(trigger, { trigger = it }, "When I say", placeholder = "my address")
                        Spacer(Modifier.height(8.dp))
                        Field(expansion, { expansion = it }, "Write", singleLine = false, placeholder = "12 Marine Drive, Kochi 682031")
                        Spacer(Modifier.height(10.dp))
                        PrimaryButton("Add snippet", {
                            model.run { snippetUpsert(trigger, expansion) }
                            trigger = ""
                            expansion = ""
                        }, enabled = trigger.isNotBlank() && expansion.isNotBlank())
                    }
                }
                item { Muted("A snippet is used when the whole dictation is just its trigger, and skips the AI.") }
                items(model.snippets, key = { "n${it.id}" }) { snippet ->
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(vertical = 4.dp)) {
                        Column(Modifier.weight(1f)) {
                            Text("\"${snippet.trigger}\"", style = HamsType.bodyStrong, color = c.text)
                            Text(snippet.expansion, style = HamsType.small, color = c.muted, maxLines = 2)
                        }
                        ConfirmLink("Remove", "Remove?") { model.run { snippetDelete(snippet.id) } }
                    }
                    Rule()
                }
            }
        }
        item { Spacer(Modifier.height(16.dp)) }
    }
}
