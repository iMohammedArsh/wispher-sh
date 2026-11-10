package com.wusla.hams.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.Spacer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.wusla.hams.core.Parse
import com.wusla.hams.core.Run
import com.wusla.hams.core.RunKind
import com.wusla.hams.core.WordDiff
import com.wusla.hams.core.changeSummary
import com.wusla.hams.core.wordDiffJson
import com.wusla.hams.ui.theme.HamsColors
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens

/**
 * The signature element (desktop: components/HeardWritten.svelte): what you
 * said, small and muted, above what Hams wrote, which is the big text. Dropped
 * words are struck through; changed or added words get a mint highlight. The
 * diff comes from the engine (`text::diff_words`), so every app marks the
 * same words.
 */
@Composable
fun HeardWritten(
    raw: String,
    text: String,
    modifier: Modifier = Modifier,
    compact: Boolean = false,
    summary: Boolean = false,
) {
    val colors = Tokens.colors
    val diff = remember(raw, text) { diffOf(raw, text) }
    val same = raw.isBlank() || (diff.dropped == 0 && diff.added == 0)
    Column(modifier, verticalArrangement = Arrangement.spacedBy(if (compact) 4.dp else 8.dp)) {
        if (!same) {
            if (!compact) Text("You said", style = HamsType.small, color = colors.faint)
            Text(
                runs(diff.heard, colors),
                style = HamsType.small.copy(fontSize = if (compact) 13.sp else 14.sp), color = colors.muted,
                maxLines = if (compact) 1 else Int.MAX_VALUE, overflow = TextOverflow.Ellipsis,
            )
            if (!compact) Spacer(Modifier.height(2.dp))
        }
        if (!compact && !same) Text("Hams wrote", style = HamsType.small, color = colors.faint)
        Text(
            runs(diff.written, colors),
            style = if (compact) HamsType.written.copy(fontSize = 16.sp, lineHeight = 23.sp) else HamsType.written.copy(fontSize = 19.sp, lineHeight = 27.sp),
            color = colors.text,
            maxLines = if (compact) 3 else Int.MAX_VALUE, overflow = TextOverflow.Ellipsis,
        )
        if (summary && !compact) {
            val note = when {
                !same -> remember(raw, text) { changeSummary(raw, text) }
                raw.isNotBlank() && raw.trim() != text.trim() -> "Written as heard, with punctuation and capitals added."
                else -> "Written exactly as heard."
            }
            Text(note, style = HamsType.small, color = colors.data)
        }
    }
}

private fun diffOf(raw: String, text: String): WordDiff =
    try {
        Parse.diff(wordDiffJson(raw, text))
    } catch (_: Throwable) {
        // The engine library failed to load: show the text without marks.
        WordDiff(listOf(Run(RunKind.KEEP, raw.split(" "))), listOf(Run(RunKind.KEEP, text.split(" "))), 0, 0)
    }

private fun runs(runs: List<Run>, colors: HamsColors): AnnotatedString = buildAnnotatedString {
    var first = true
    for (run in runs) {
        val style = when (run.kind) {
            RunKind.KEEP -> null
            RunKind.DROP -> SpanStyle(color = colors.faint, textDecoration = TextDecoration.LineThrough)
            RunKind.ADD -> SpanStyle(color = colors.text, background = colors.dataSoft)
        }
        for (word in run.words) {
            if (word == "\n") {
                append("\n")
                first = true
                continue
            }
            if (!first) append(" ")
            first = false
            if (style == null) append(word) else withStyle(style) { append(word) }
        }
    }
}
