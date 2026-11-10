package com.wusla.hams.ui.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.text.format.DateUtils
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.wusla.hams.R
import com.wusla.hams.core.HistoryItem
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.components.ConfirmLink
import com.wusla.hams.ui.components.QuietButton
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens

fun copyText(context: Context, text: String) {
    context.getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText("Hams", text))
}

fun relativeTime(ms: Long): String =
    DateUtils.getRelativeTimeSpanString(ms, System.currentTimeMillis(), DateUtils.MINUTE_IN_MILLIS).toString()

/** Copy · Rewrite · Pin · Delete, for one dictation. */
@Composable
fun DictationActions(model: AppModel, item: HistoryItem, showDelete: Boolean = true) {
    val context = LocalContext.current
    val c = Tokens.colors
    var menu by remember { mutableStateOf(false) }
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        itemVerticalAlignment = Alignment.CenterVertically,
    ) {
        QuietButton("Copy", { copyText(context, item.text); model.message = "Copied" }, icon = R.drawable.ic_copy)
        Box {
            QuietButton("Rewrite", { menu = true }, icon = R.drawable.ic_wand)
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }, containerColor = c.surface) {
                model.styles.forEach { style ->
                    DropdownMenuItem(
                        text = { Text(style.label, style = HamsType.body, color = c.text) },
                        onClick = {
                            menu = false
                            model.run("Rewritten as ${style.label}") { historyRestyle(item.id, style.id) }
                        },
                    )
                }
            }
        }
        QuietButton(if (item.pinned) "Unpin" else "Pin", { model.run { historyPin(item.id, !item.pinned) } }, icon = R.drawable.ic_pin)
        if (showDelete) ConfirmLink("Delete", "Tap again to delete") { model.run { historyDelete(item.id) } }
    }
}
