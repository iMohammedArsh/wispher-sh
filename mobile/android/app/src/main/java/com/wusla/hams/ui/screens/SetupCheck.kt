package com.wusla.hams.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.Pill
import com.wusla.hams.ui.components.Rule
import com.wusla.hams.ui.components.StepDot
import com.wusla.hams.ui.components.Tone
import com.wusla.hams.ui.components.bounceClick
import com.wusla.hams.ui.micPromptCanBeShownAgain
import com.wusla.hams.ui.openAppSettings
import com.wusla.hams.ui.openKeyboardSettings
import com.wusla.hams.ui.showKeyboardPicker
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens

private class Check(val title: String, val hint: String, val done: Boolean, val fix: () -> Unit)

/**
 * Everything Hams needs to type for you, one row each. A row that isn't done
 * says what to do and fixes it when tapped: Home shows this while setup is
 * unfinished, and You → Setup check always does, for when something stops working.
 */
@Composable
fun SetupChecklist(
    model: AppModel,
    requestMic: ((Boolean) -> Unit) -> Unit,
    openArea: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val c = Tokens.colors
    val setup = model.setup
    val pack = model.models.firstOrNull { it.id == model.settings?.model }
    val checks = listOf(
        Check("The Hams keyboard is on", "Turn it on in Android's keyboard settings.", setup.keyboardEnabled) { openKeyboardSettings(context) },
        Check(
            "Hams is the keyboard in use",
            if (setup.keyboardEnabled) "Choose Hams from the list." else "Turn the keyboard on first.",
            setup.keyboardSelected,
        ) { if (setup.keyboardEnabled) showKeyboardPicker(context) else openKeyboardSettings(context) },
        Check("Microphone allowed", "It's only on while you dictate.", setup.micAllowed) {
            requestMic { granted ->
                model.refreshSetup()
                // Refused for good: Android won't show the prompt, so go where it can be turned on.
                if (!granted && !micPromptCanBeShownAgain(context)) openAppSettings(context)
            }
        },
        Check("Speech pack downloaded", pack?.let { "${it.label}, ${it.sizeMb} MB." } ?: "Hams needs one to hear you.", setup.modelReady) { openArea("model") },
    )
    Panel(modifier, padding = PaddingValues(horizontal = 16.dp, vertical = 6.dp)) {
        checks.forEachIndexed { index, check ->
            if (index > 0) Rule()
            Row(
                Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).bounceClick(enabled = !check.done, onClick = check.fix).padding(vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                StepDot(index + 1, check.done)
                Spacer(Modifier.width(14.dp))
                Column(Modifier.weight(1f)) {
                    Text(check.title, style = HamsType.written.copy(fontWeight = FontWeight.Medium), color = c.text)
                    if (!check.done) Text(check.hint, style = HamsType.small, color = c.muted)
                }
                if (!check.done) Pill("Fix", Tone.Warning)
            }
        }
    }
}
