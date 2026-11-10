package com.wusla.hams.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.wusla.hams.R
import com.wusla.hams.core.AI_PROVIDERS
import com.wusla.hams.core.LANGUAGES
import com.wusla.hams.core.Stats
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.SectionTitle
import com.wusla.hams.ui.components.ValueRow
import com.wusla.hams.ui.openKeyboardSettings
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens
import java.text.NumberFormat
import java.time.LocalDate

/** You: what Hams has written for you, your words, and every setting, one tap deep. */
@Composable
fun YouIndex(model: AppModel, setArea: (String) -> Unit) {
    val context = LocalContext.current
    val c = Tokens.colors
    val s = model.settings ?: return
    val modelLabel = model.models.firstOrNull { it.id == s.model }?.let { if (it.installed) it.label else "${it.label} · not downloaded" } ?: s.model
    val language = LANGUAGES.firstOrNull { it.first == s.language }?.second ?: s.language
    val provider = AI_PROVIDERS.firstOrNull { it.first == s.aiProvider }?.second?.substringBefore(" (") ?: s.aiProvider

    Text("You", style = HamsType.display.copy(fontSize = HamsType.title.fontSize * 1.15f), color = c.text, modifier = Modifier.padding(top = 12.dp, bottom = 16.dp))

    model.stats?.let { StatsCard(it) }

    SectionTitle("Make it yours")
    Panel(padding = PaddingValues(horizontal = 16.dp, vertical = 6.dp)) {
        ValueRow("Your words", "", { setArea("words") }, "Names, fixes and snippets", icon = R.drawable.ic_book, badge = model.suggestions.size)
        ValueRow("Styles", if (s.appStyles) "Per app" else "One style", { setArea("styles") }, icon = R.drawable.ic_styles)
        ValueRow("AI polish", if (s.aiProvider == "off") "Off" else provider, { setArea("ai") }, icon = R.drawable.ic_sparkle)
    }

    SectionTitle("Speech")
    Panel(padding = PaddingValues(horizontal = 16.dp, vertical = 6.dp)) {
        ValueRow("Speech model", modelLabel, { setArea("model") }, "Runs on this phone", icon = R.drawable.ic_mic)
        ValueRow("Language and clean-up", language, { setArea("speech") }, icon = R.drawable.ic_globe)
    }

    SectionTitle("Phone")
    Panel(padding = PaddingValues(horizontal = 16.dp, vertical = 6.dp)) {
        ValueRow("Keyboard", "", { openKeyboardSettings(context) }, "Turn Hams on or off in Android settings", icon = R.drawable.ic_keyboard)
        ValueRow("Privacy", "", { setArea("privacy") }, "Everything stays on this phone", icon = R.drawable.ic_shield)
    }

    Text(
        "Hams by Wusla", style = HamsType.small, color = c.faint,
        modifier = Modifier.fillMaxWidth().padding(top = 28.dp), textAlign = androidx.compose.ui.text.style.TextAlign.Center,
    )
}

/** Three numbers and the last sixteen weeks, one square a day. */
@Composable
private fun StatsCard(stats: Stats) {
    val c = Tokens.colors
    val number = remember { NumberFormat.getIntegerInstance() }
    Panel(color = c.primary.takeIf { !c.dark } ?: c.surface, padding = PaddingValues(20.dp)) {
        val ink = if (c.dark) c.text else c.primaryInk
        val soft = if (c.dark) c.muted else c.primaryInk.copy(alpha = 0.7f)
        Row(Modifier.fillMaxWidth()) {
            Figure(number.format(stats.totalWords), "words written", ink, soft, Modifier.weight(1f))
            Figure("${stats.minutesSaved}", "minutes saved", ink, soft, Modifier.weight(1f))
            Figure("${stats.currentStreak}", "day streak", ink, soft, Modifier.weight(1f))
        }
        Spacer(Modifier.height(18.dp))
        Activity(stats.activity)
    }
}

@Composable
private fun Figure(value: String, label: String, ink: androidx.compose.ui.graphics.Color, soft: androidx.compose.ui.graphics.Color, modifier: Modifier) {
    Column(modifier) {
        Text(value, style = HamsType.number, color = ink)
        Text(label, style = HamsType.small, color = soft)
    }
}

/** 16 weeks × 7 days. Empty days are faint; busier days glow mint. */
@Composable
private fun Activity(activity: Map<String, Long>) {
    val c = Tokens.colors
    val today = remember { LocalDate.now() }
    // Align columns to weeks starting Monday, ending with this week.
    val start = remember(today) { today.minusDays((today.dayOfWeek.value - 1).toLong()).minusWeeks(WEEKS - 1L) }
    val busiest = (activity.values.maxOrNull() ?: 0L).coerceAtLeast(1L)
    val empty = if (c.dark) c.active else c.primaryInk.copy(alpha = 0.12f)
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        for (week in 0 until WEEKS) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                for (day in 0 until 7) {
                    val date = start.plusDays((week * 7 + day).toLong())
                    val words = activity[date.toString()] ?: 0L
                    val color = when {
                        date.isAfter(today) -> empty.copy(alpha = empty.alpha * 0.4f)
                        words == 0L -> empty
                        else -> lerp(c.mint.copy(alpha = 0.45f), c.mint, (words.toFloat() / busiest).coerceIn(0f, 1f))
                    }
                    Box(Modifier.fillMaxWidth().aspectRatio(1f).clip(RoundedCornerShape(3.dp)).background(color))
                }
            }
        }
    }
    Spacer(Modifier.height(4.dp))
    Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.CenterEnd) {
        Text("Last $WEEKS weeks", style = HamsType.small, color = if (c.dark) c.faint else c.primaryInk.copy(alpha = 0.6f))
    }
}

private const val WEEKS = 16
