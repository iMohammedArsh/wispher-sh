package com.wusla.hams.ui.screens

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.wusla.hams.core.Hams
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.TryField
import com.wusla.hams.ui.TryFieldLifecycle
import com.wusla.hams.ui.components.HeardWritten
import com.wusla.hams.ui.components.LinkButton
import com.wusla.hams.ui.components.Orb
import com.wusla.hams.ui.components.OrbMood
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.PrimaryButton
import com.wusla.hams.ui.components.QuietButton
import com.wusla.hams.ui.holdToTalk
import com.wusla.hams.ui.openAppSettings
import com.wusla.hams.ui.openKeyboardSettings
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens

private enum class Step { Welcome, Keyboard, Mic, Model, Try }

/**
 * First run, one thing per screen: hello, turn on the keyboard, allow the
 * mic, get the model, then hold the orb and see what Hams wrote. Steps that
 * are already done are skipped; coming back from Android settings moves on
 * by itself.
 */
@Composable
fun OnboardingScreen(model: AppModel, requestMic: ((Boolean) -> Unit) -> Unit, onDone: () -> Unit) {
    val setup = model.setup
    var step by rememberSaveable { mutableStateOf(Step.Welcome) }
    var micDenied by remember { mutableStateOf(false) }
    TryFieldLifecycle()

    fun nextAfter(current: Step): Step = when {
        current < Step.Keyboard && !setup.keyboardEnabled -> Step.Keyboard
        current < Step.Mic && !setup.micAllowed -> Step.Mic
        current < Step.Model && !setup.modelReady -> Step.Model
        else -> Step.Try
    }

    // A step got done (often in Android settings): move on.
    LaunchedEffect(setup, step) {
        val done = when (step) {
            Step.Keyboard -> setup.keyboardEnabled
            Step.Mic -> setup.micAllowed
            Step.Model -> setup.modelReady
            else -> false
        }
        if (done) step = nextAfter(step)
    }

    val c = Tokens.colors
    Column(
        Modifier
            .fillMaxSize()
            .background(c.ground)
            .windowInsetsPadding(WindowInsets.safeDrawing)
            .padding(horizontal = 24.dp, vertical = 12.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth().height(48.dp)) {
            Dots(step.ordinal, Step.entries.size, Modifier.weight(1f))
            if (step != Step.Try) LinkButton("Skip", onDone, color = c.muted)
        }

        AnimatedContent(
            step,
            transitionSpec = {
                (slideInHorizontally(tween(320)) { it / 6 } + fadeIn(tween(320))) togetherWith
                    (slideOutHorizontally(tween(200)) { -it / 6 } + fadeOut(tween(160)))
            },
            modifier = Modifier.weight(1f),
            label = "step",
        ) { current ->
            when (current) {
                Step.Welcome -> Page(
                    visual = { Orb(OrbMood.Idle, 0f, diameter = 260.dp) },
                    title = "Talk, and it's written.",
                    body = "Hold the orb and speak like you normally would. Hams takes out the ums, fixes the punctuation and types it for you. Your voice never leaves this phone.",
                ) { PrimaryButton("Get started", { step = nextAfter(Step.Welcome) }, Modifier.fillMaxWidth()) }

                Step.Keyboard -> Page(
                    visual = { Orb(OrbMood.Idle, 0f, diameter = 200.dp) },
                    title = "Turn on the Hams keyboard",
                    body = "Android asks you to confirm, because any keyboard can see what you type. Hams only reads a few letters around your cursor, to space words properly.",
                ) {
                    val context = LocalContext.current
                    PrimaryButton("Open keyboard settings", { openKeyboardSettings(context) }, Modifier.fillMaxWidth())
                }

                Step.Mic -> Page(
                    visual = { Orb(OrbMood.Idle, 0f, diameter = 200.dp) },
                    title = "Let Hams hear you",
                    body = "The microphone is on only while you hold the orb. Nothing is recorded in the background.",
                ) {
                    val context = LocalContext.current
                    if (micDenied) {
                        Text("Android won't ask again. Allow it in Hams's app settings → Permissions.", style = HamsType.body, color = c.muted, textAlign = TextAlign.Center)
                        Spacer(Modifier.height(12.dp))
                        PrimaryButton("Open app settings", { openAppSettings(context) }, Modifier.fillMaxWidth())
                    } else {
                        PrimaryButton("Allow microphone", {
                            requestMic { granted ->
                                micDenied = !granted
                                model.refreshSetup()
                            }
                        }, Modifier.fillMaxWidth())
                    }
                }

                Step.Model -> ModelPage(model)

                Step.Try -> TryPage(model, onDone)
            }
        }
    }
}

@Composable
private fun Dots(current: Int, count: Int, modifier: Modifier) {
    val c = Tokens.colors
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        for (i in 0 until count) {
            val width by animateDpAsState(if (i == current) 24.dp else 8.dp, label = "dot")
            Box(Modifier.size(width, 8.dp).clip(RoundedCornerShape(50)).background(if (i <= current) c.primary else c.active))
        }
    }
}

/** Visual on top, a big line and a sentence, the action at the bottom. */
@Composable
private fun Page(
    visual: @Composable () -> Unit,
    title: String,
    body: String,
    actions: @Composable () -> Unit,
) {
    val c = Tokens.colors
    Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) { visual() }
        Text(title, style = HamsType.display, color = c.text, textAlign = TextAlign.Center)
        Spacer(Modifier.height(14.dp))
        Text(body, style = HamsType.written, color = c.muted, textAlign = TextAlign.Center)
        Spacer(Modifier.height(32.dp))
        Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) { actions() }
        Spacer(Modifier.height(12.dp))
    }
}

@Composable
private fun ModelPage(model: AppModel) {
    val c = Tokens.colors
    val chosen = model.settings?.model
    val progress = chosen?.let { model.downloads[it] }
    val info = model.models.firstOrNull { it.id == chosen }
    Page(
        visual = {
            Box(contentAlignment = Alignment.Center) {
                if (progress != null) {
                    CircularProgressIndicator(
                        progress = { progress }, modifier = Modifier.size(236.dp),
                        color = c.data, trackColor = c.active, strokeWidth = 5.dp, strokeCap = StrokeCap.Round, gapSize = 0.dp,
                    )
                }
                Orb(if (progress != null) OrbMood.Working else OrbMood.Idle, 0f, diameter = 220.dp)
            }
        },
        title = "Get the speech model",
        body = (info?.let { "${it.label}, ${it.sizeMb} MB, downloaded once. " } ?: "Downloaded once. ") +
            "After that, everything runs on this phone, even offline.",
    ) {
        if (progress != null) {
            Text("Downloading… ${(progress * 100).toInt()}%", style = HamsType.section, color = c.text)
        } else if (chosen != null) {
            PrimaryButton("Download", { model.download(chosen) }, Modifier.fillMaxWidth())
        }
    }
}

@Composable
private fun TryPage(model: AppModel, onDone: () -> Unit) {
    val c = Tokens.colors
    val status by Hams.status.collectAsState()
    val level by Hams.level.collectAsState()
    val tried by TryField.text.collectAsState()
    val ready = model.setup.micAllowed && model.setup.modelReady
    // The newest history item, once something was dictated on this page.
    val last = model.recent.firstOrNull()?.takeIf { tried.isNotBlank() }

    Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
            Box(Modifier.size(240.dp).holdToTalk(ready), contentAlignment = Alignment.Center) {
                Orb(orbMood(status.phase, ready), level, diameter = 240.dp)
            }
        }
        Text(if (last == null) "Now say something" else "That's Hams.", style = HamsType.display, color = c.text, textAlign = TextAlign.Center)
        Spacer(Modifier.height(14.dp))
        AnimatedVisibility(last == null) {
            Text(
                if (ready) "Hold the orb and talk. Let go when you're done. Try \"um so I'll send it over tomorrow morning\"."
                else "Finish the steps before to try it.",
                style = HamsType.written, color = c.muted, textAlign = TextAlign.Center,
            )
        }
        if (last != null) {
            Panel(modifier = Modifier.animateContentSize(), padding = PaddingValues(18.dp)) {
                HeardWritten(last.raw, last.text, summary = true)
            }
        }
        Spacer(Modifier.height(24.dp))
        if (last != null) {
            PrimaryButton("Start using Hams", onDone, Modifier.fillMaxWidth())
            Spacer(Modifier.height(4.dp))
            QuietButton("Try again", { TryField.clear() })
        } else {
            LinkButton("Skip for now", onDone, color = c.muted)
        }
        Spacer(Modifier.height(12.dp))
    }
}
