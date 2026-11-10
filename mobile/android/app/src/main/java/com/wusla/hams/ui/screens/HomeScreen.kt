package com.wusla.hams.ui.screens

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.wusla.hams.R
import com.wusla.hams.core.Hams
import com.wusla.hams.core.HistoryItem
import com.wusla.hams.core.Parse
import com.wusla.hams.core.Phase
import com.wusla.hams.core.io
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.TryField
import com.wusla.hams.ui.TryFieldLifecycle
import com.wusla.hams.ui.components.ConfirmLink
import com.wusla.hams.ui.components.Field
import com.wusla.hams.ui.components.HamsIcon
import com.wusla.hams.ui.components.HeardWritten
import com.wusla.hams.ui.components.Muted
import com.wusla.hams.ui.components.Orb
import com.wusla.hams.ui.components.OrbMood
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.Pill
import com.wusla.hams.ui.components.PrimaryButton
import com.wusla.hams.ui.components.QuietButton
import com.wusla.hams.ui.components.RoundIcon
import com.wusla.hams.ui.components.SectionTitle
import com.wusla.hams.ui.components.bounceClick
import com.wusla.hams.ui.holdToTalk
import com.wusla.hams.ui.openKeyboardSettings
import com.wusla.hams.ui.showKeyboardPicker
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens
import kotlinx.coroutines.delay
import java.text.NumberFormat
import java.util.Calendar

/**
 * Home: a greeting, the orb (hold it to dictate right here), and what you've
 * said lately. The search button turns the list into a search over everything.
 */
@Composable
fun HomeScreen(model: AppModel, requestMic: ((Boolean) -> Unit) -> Unit, openSettings: (String) -> Unit) {
    var searching by rememberSaveable { mutableStateOf(false) }
    var query by rememberSaveable { mutableStateOf("") }
    var found by remember { mutableStateOf<List<HistoryItem>>(emptyList()) }
    var expanded by remember { mutableLongStateOf(-1L) }
    val version by Hams.dataVersion.collectAsState()
    TryFieldLifecycle()

    LaunchedEffect(searching, query, version) {
        if (!searching) return@LaunchedEffect
        delay(if (query.isEmpty()) 0 else 200)
        io { Parse.history(historyJson(query, 300)) }.onSuccess { found = it }
    }
    val list = if (searching) found else model.recent

    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = 20.dp, end = 20.dp, top = 12.dp, bottom = 120.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            AnimatedContent(searching, transitionSpec = { fadeIn() togetherWith fadeOut() }, label = "header") { search ->
                if (search) {
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(bottom = 4.dp)) {
                        Field(query, { query = it }, "", modifier = Modifier.weight(1f), placeholder = "Search what you've said")
                        Spacer(Modifier.width(10.dp))
                        RoundIcon(R.drawable.ic_close, { searching = false; query = "" }, description = "Close search")
                    }
                } else {
                    Greeting(model) { searching = true }
                }
            }
        }

        if (!searching) {
            item { Hero(model, requestMic, openSettings) }
            if (list.isNotEmpty()) item { SectionTitle("Recent") }
        }

        if (list.isEmpty() && searching) {
            item { Muted(if (query.isEmpty()) "Nothing here yet." else "Nothing matches \"$query\".", Modifier.padding(8.dp)) }
        }
        items(list, key = { it.id }) { item ->
            DictationCard(model, item, open = expanded == item.id) { expanded = if (expanded == item.id) -1L else item.id }
        }
        if (searching && query.isEmpty() && list.isNotEmpty()) {
            item {
                Box(Modifier.fillMaxWidth().padding(top = 8.dp), contentAlignment = Alignment.Center) {
                    ConfirmLink("Clear history (keeps pinned)", "Tap again to clear") { model.run("History cleared") { historyClear() } }
                }
            }
        }
    }
}

@Composable
private fun Greeting(model: AppModel, onSearch: () -> Unit) {
    val c = Tokens.colors
    val stats = model.stats
    Row(verticalAlignment = Alignment.Top, modifier = Modifier.padding(top = 12.dp, bottom = 4.dp)) {
        Column(Modifier.weight(1f)) {
            Text(greeting(), style = HamsType.display.copy(fontSize = HamsType.title.fontSize * 1.15f), color = c.text)
            if (stats != null && stats.totalWords > 0) {
                Spacer(Modifier.height(6.dp))
                val number = NumberFormat.getIntegerInstance()
                Text(
                    buildAnnotatedString {
                        withStyle(SpanStyle(fontWeight = FontWeight.SemiBold, color = c.text)) { append(number.format(stats.totalWords)) }
                        append(" words written")
                        if (stats.minutesSaved > 0) {
                            append(" · ")
                            withStyle(SpanStyle(fontWeight = FontWeight.SemiBold, color = c.text)) { append("${stats.minutesSaved} min") }
                            append(" saved")
                        }
                        if (stats.currentStreak > 1) append(" · ${stats.currentStreak}-day streak")
                    },
                    style = HamsType.body, color = c.muted,
                )
            }
        }
        if (model.recent.isNotEmpty()) RoundIcon(R.drawable.ic_search, onSearch, description = "Search")
    }
}

private fun greeting(): String = when (Calendar.getInstance().get(Calendar.HOUR_OF_DAY)) {
    in 5..11 -> "Good morning"
    in 12..16 -> "Good afternoon"
    in 17..21 -> "Good evening"
    else -> "Hello, night owl"
}

/** The orb, or the one setup step still missing. */
@Composable
private fun Hero(model: AppModel, requestMic: ((Boolean) -> Unit) -> Unit, openSettings: (String) -> Unit) {
    val context = LocalContext.current
    val c = Tokens.colors
    val setup = model.setup
    val status by Hams.status.collectAsState()
    val level by Hams.level.collectAsState()
    val partial by Hams.partial.collectAsState()
    val tried by TryField.text.collectAsState()
    val ready = setup.micAllowed && setup.modelReady
    val mood = orbMood(status.phase, ready)

    Panel(padding = PaddingValues(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 20.dp)) {
        Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
            Box(
                Modifier.size(232.dp).holdToTalk(ready) {
                    if (!setup.micAllowed) requestMic { model.refreshSetup() } else openSettings("model")
                },
                contentAlignment = Alignment.Center,
            ) { Orb(mood, level, diameter = 232.dp) }

            AnimatedContent(Triple(status.phase, ready, status.locked), transitionSpec = { fadeIn() togetherWith fadeOut() }, label = "hint") { (phase, ok, locked) ->
                Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.heightIn(min = 64.dp)) {
                    val (title, line) = when {
                        !setup.micAllowed -> "Let Hams hear you" to "Tap the orb to allow the microphone."
                        !setup.modelReady -> "One download left" to "Tap the orb to get the speech model."
                        phase == Phase.RECORDING -> (if (locked) "Listening · tap to finish" else "Listening…") to "Let go when you're done."
                        phase == Phase.TRANSCRIBING || phase == Phase.POLISHING || phase == Phase.DELIVERING -> "Writing it down…" to " "
                        else -> "Hold to talk" to "Try it right here. Say anything, then let go."
                    }
                    Text(title, style = HamsType.heading, color = c.text, textAlign = TextAlign.Center)
                    Spacer(Modifier.height(4.dp))
                    Text(line, style = HamsType.body, color = c.muted, textAlign = TextAlign.Center)
                    if (ok && phase == Phase.RECORDING && partial.isNotBlank()) {
                        Spacer(Modifier.height(6.dp))
                        Text(partial, style = HamsType.body, color = c.text, maxLines = 2, overflow = TextOverflow.Ellipsis, textAlign = TextAlign.Center)
                    }
                }
            }

            AnimatedVisibility(tried.isNotBlank(), enter = expandVertically() + fadeIn(), exit = shrinkVertically() + fadeOut()) {
                Column(Modifier.padding(top = 16.dp).fillMaxWidth()) {
                    Panel(color = c.dataSoft, padding = PaddingValues(16.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("Just now", style = HamsType.small, color = c.data, modifier = Modifier.weight(1f))
                            Text(
                                "Clear", style = HamsType.label, color = c.data,
                                modifier = Modifier.bounceClick { TryField.clear() }.padding(4.dp),
                            )
                        }
                        Spacer(Modifier.height(6.dp))
                        Text(tried.trim(), style = HamsType.written, color = c.text, modifier = Modifier.animateContentSize())
                    }
                }
            }

            if (ready && (!setup.keyboardEnabled || !setup.keyboardSelected)) {
                Spacer(Modifier.height(16.dp))
                KeyboardNudge(setup.keyboardEnabled, { openKeyboardSettings(context) }, { showKeyboardPicker(context) })
            }
        }
    }
}

@Composable
private fun KeyboardNudge(enabled: Boolean, turnOn: () -> Unit, pick: () -> Unit) {
    val c = Tokens.colors
    Row(
        Modifier.fillMaxWidth().padding(top = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(40.dp).clip(CircleShape), contentAlignment = Alignment.Center) {
            HamsIcon(R.drawable.ic_keyboard, tint = c.muted)
        }
        Spacer(Modifier.width(10.dp))
        Column(Modifier.weight(1f)) {
            Text("Use it in every app", style = HamsType.bodyStrong, color = c.text)
            Text(
                if (enabled) "Switch to the Hams keyboard while typing." else "Turn on the Hams keyboard.",
                style = HamsType.small, color = c.muted,
            )
        }
        Spacer(Modifier.width(8.dp))
        if (enabled) QuietButton("Switch", pick) else PrimaryButton("Turn on", turnOn)
    }
}

fun orbMood(phase: Phase, ready: Boolean): OrbMood = when {
    !ready -> OrbMood.Off
    phase == Phase.RECORDING -> OrbMood.Listening
    phase == Phase.TRANSCRIBING || phase == Phase.POLISHING || phase == Phase.DELIVERING -> OrbMood.Working
    phase == Phase.DONE -> OrbMood.Done
    phase == Phase.ERROR -> OrbMood.Error
    else -> OrbMood.Idle
}

/** One dictation: the app and when, then what you said → what Hams wrote. Tap to open its actions. */
@Composable
fun DictationCard(model: AppModel, item: HistoryItem, open: Boolean, onToggle: () -> Unit) {
    val c = Tokens.colors
    Panel(modifier = Modifier.bounceClick(onClick = onToggle).animateContentSize(), padding = PaddingValues(18.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(8.dp).clip(CircleShape).background(c.voice))
            Spacer(Modifier.width(8.dp))
            Text(
                "${item.app.ifEmpty { "Dictation" }} · ${relativeTime(item.createdAt)}",
                style = HamsType.small, color = c.muted, modifier = Modifier.weight(1f),
            )
            if (item.pinned) Pill("Pinned")
        }
        Spacer(Modifier.height(10.dp))
        HeardWritten(item.raw, item.text, compact = !open, summary = open)
        if (open) {
            Spacer(Modifier.height(14.dp))
            DictationActions(model, item)
        }
    }
}
