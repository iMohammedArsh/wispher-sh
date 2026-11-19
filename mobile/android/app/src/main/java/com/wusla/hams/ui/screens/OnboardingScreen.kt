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
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.runtime.mutableLongStateOf
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
import androidx.compose.ui.window.Dialog
import com.wusla.hams.core.Hams
import com.wusla.hams.core.POPULAR_LANGUAGES
import com.wusla.hams.ui.AppModel
import com.wusla.hams.ui.TryField
import com.wusla.hams.ui.TryFieldLifecycle
import com.wusla.hams.ui.components.Chip
import com.wusla.hams.ui.components.Field
import com.wusla.hams.ui.components.HeardWritten
import com.wusla.hams.ui.components.LinkButton
import com.wusla.hams.ui.components.Orb
import com.wusla.hams.ui.components.OrbMood
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.PrimaryButton
import com.wusla.hams.ui.components.QuietButton
import com.wusla.hams.ui.MicAsk
import com.wusla.hams.ui.holdToTalk
import com.wusla.hams.ui.micAskAfterRefusal
import com.wusla.hams.ui.micPromptCanBeShownAgain
import com.wusla.hams.ui.openAppSettings
import com.wusla.hams.ui.openKeyboardSettings
import com.wusla.hams.ui.showKeyboardPicker
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens

private enum class Step { Welcome, Language, Keyboard, Select, Mic, Model, Try }

/**
 * First run, one thing per screen: hello, the languages you speak, turn on
 * the keyboard, switch to it, allow the mic, get the speech pack that suits
 * your languages, then dictate and see what Hams wrote. Steps that are already
 * done are skipped; coming back from Android settings moves on by itself.
 */
@Composable
fun OnboardingScreen(model: AppModel, requestMic: ((Boolean) -> Unit) -> Unit, onDone: () -> Unit) {
    val setup = model.setup
    var step by rememberSaveable { mutableStateOf(Step.Welcome) }
    var micAsk by rememberSaveable { mutableStateOf(MicAsk.Fresh) }
    var confirmSkip by remember { mutableStateOf(false) }
    TryFieldLifecycle()

    fun nextAfter(current: Step): Step = when {
        current < Step.Language && model.settings?.languages.isNullOrEmpty() -> Step.Language
        current < Step.Keyboard && !setup.keyboardEnabled -> Step.Keyboard
        current < Step.Select && !setup.keyboardSelected -> Step.Select
        current < Step.Mic && !setup.micAllowed -> Step.Mic
        current < Step.Model && !setup.modelReady -> Step.Model
        else -> Step.Try
    }

    // A step got done (often in Android settings): move on.
    LaunchedEffect(setup, step) {
        val done = when (step) {
            Step.Keyboard -> setup.keyboardEnabled
            Step.Select -> setup.keyboardSelected
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
            // Skipping with things missing leaves a keyboard that can't type: say so first.
            if (step != Step.Try) LinkButton("Skip", { if (setup.complete) onDone() else confirmSkip = true }, color = c.muted)
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

                Step.Language -> LanguagePage(model) { step = nextAfter(Step.Language) }

                Step.Keyboard -> Page(
                    visual = { Orb(OrbMood.Idle, 0f, diameter = 200.dp) },
                    title = "Turn on the Hams keyboard",
                    body = "Android asks you to confirm, because any keyboard can see what you type. Hams only reads a few letters around your cursor, to space words properly.",
                ) {
                    val context = LocalContext.current
                    PrimaryButton("Open keyboard settings", { openKeyboardSettings(context) }, Modifier.fillMaxWidth())
                }

                Step.Select -> Page(
                    visual = { Orb(OrbMood.Idle, 0f, diameter = 200.dp) },
                    title = "Switch to Hams",
                    body = "Turning it on isn't enough: Android types with one keyboard at a time. Pick Hams from the list. You can switch back any time with the globe key.",
                ) {
                    val context = LocalContext.current
                    PrimaryButton("Choose Hams", { showKeyboardPicker(context) }, Modifier.fillMaxWidth())
                }

                Step.Mic -> Page(
                    visual = { Orb(OrbMood.Idle, 0f, diameter = 200.dp) },
                    title = "Let Hams hear you",
                    body = "The microphone is on only while you hold the orb. Nothing is recorded in the background.",
                ) {
                    val context = LocalContext.current
                    if (micAsk == MicAsk.Blocked) {
                        Text("Android won't ask again. Allow it in Hams's app settings → Permissions.", style = HamsType.body, color = c.muted, textAlign = TextAlign.Center)
                        Spacer(Modifier.height(12.dp))
                        PrimaryButton("Open app settings", { openAppSettings(context) }, Modifier.fillMaxWidth())
                    } else {
                        if (micAsk == MicAsk.Retry) {
                            Text("Hams can't hear you without it. It only listens while you hold the orb.", style = HamsType.body, color = c.muted, textAlign = TextAlign.Center)
                            Spacer(Modifier.height(12.dp))
                        }
                        PrimaryButton("Allow microphone", {
                            requestMic { granted ->
                                // Read the rationale now, right after the refusal: before the first prompt it is also false.
                                micAsk = if (granted) MicAsk.Fresh else micAskAfterRefusal(micPromptCanBeShownAgain(context))
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
    if (confirmSkip) SkipDialog(onSkip = onDone, onStay = { confirmSkip = false })
}

@Composable
private fun SkipDialog(onSkip: () -> Unit, onStay: () -> Unit) {
    val c = Tokens.colors
    Dialog(onDismissRequest = onStay) {
        Panel {
            Text("Skip setup?", style = HamsType.heading, color = c.text)
            Spacer(Modifier.height(8.dp))
            Text(
                "Hams won't type anything until its keyboard is on and chosen, the microphone is allowed and a speech pack is downloaded. Home keeps a checklist, so you can finish later.",
                style = HamsType.body, color = c.muted,
            )
            Spacer(Modifier.height(20.dp))
            PrimaryButton("Keep going", onStay, Modifier.fillMaxWidth())
            Spacer(Modifier.height(4.dp))
            QuietButton("Skip anyway", onSkip, Modifier.fillMaxWidth())
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

/** "What do you speak?": the phone's languages are picked already; more from the full list. */
@Composable
private fun LanguagePage(model: AppModel, onNext: () -> Unit) {
    val c = Tokens.colors
    var picked by remember { mutableStateOf<List<String>>(emptyList()) }
    var more by remember { mutableStateOf(false) }
    LaunchedEffect(model.languages) {
        if (picked.isEmpty() && model.languages.isNotEmpty()) {
            picked = model.settings?.languages.orEmpty().ifEmpty { model.deviceLanguages().take(2).ifEmpty { listOf("en") } }
        }
    }
    val shown = (picked + model.deviceLanguages() + POPULAR_LANGUAGES).distinct().take(maxOf(12, picked.size))
    Page(
        visual = {
            FlowRow(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                shown.forEach { code ->
                    val on = code in picked
                    Chip(model.lang(code)?.native ?: code, { picked = if (on) picked - code else picked + code }, selected = on)
                }
            }
        },
        title = "What do you speak?",
        body = "Pick every language you dictate in. Hams gets the speech pack that suits them, and writes in the one you use.",
    ) {
        PrimaryButton("Continue", {
            model.saveLanguages(picked)
            onNext()
        }, Modifier.fillMaxWidth(), enabled = picked.isNotEmpty())
        Spacer(Modifier.height(4.dp))
        LinkButton("More languages", { more = true }, color = c.muted)
    }
    if (more) {
        AddLanguageSheet(model, picked, onPick = { picked = picked + it; more = false }, onDismiss = { more = false })
    }
}

@Composable
private fun ModelPage(model: AppModel) {
    val c = Tokens.colors
    val chosen = model.settings?.model
    val progress = chosen?.let { model.downloads[it] }
    val failure = chosen?.let { model.downloadFailures[it] }
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
        title = "Get your speech pack",
        body = (info?.let { "${it.label}, ${it.sizeMb} MB, downloaded once. " } ?: "Downloaded once. ") +
            "After that, everything runs on this phone, even offline.",
    ) {
        if (progress != null) {
            Text("Downloading… ${(progress * 100).toInt()}%", style = HamsType.section, color = c.text)
            Spacer(Modifier.height(4.dp))
            Text("You can leave this screen. It keeps going.", style = HamsType.small, color = c.muted)
        } else if (chosen != null) {
            if (failure != null) {
                // Stays until tried again, unlike a toast: someone away from the screen would miss it.
                Text("The download stopped: $failure", style = HamsType.body, color = c.negative, textAlign = TextAlign.Center)
                Spacer(Modifier.height(4.dp))
                Text("Hams carries on from where it stopped.", style = HamsType.small, color = c.muted, textAlign = TextAlign.Center)
                Spacer(Modifier.height(12.dp))
            }
            PrimaryButton(if (failure != null) "Try again" else "Download", { model.download(chosen) }, Modifier.fillMaxWidth())
        }
    }
}

@Composable
private fun TryPage(model: AppModel, onDone: () -> Unit) {
    val c = Tokens.colors
    val status by Hams.status.collectAsState()
    val level by Hams.level.collectAsState()
    val ready = model.setup.micAllowed && model.setup.modelReady
    // A dictation counts once it's newer than this page (or than the last "Try again"),
    // whichever way it was made: the orb here, or the keyboard in the field below.
    var since by remember { mutableLongStateOf(System.currentTimeMillis()) }
    var typed by remember { mutableStateOf("") }
    val last = model.recent.firstOrNull { it.createdAt >= since }

    Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
            Box(Modifier.size(200.dp).holdToTalk(ready), contentAlignment = Alignment.Center) {
                Orb(orbMood(status.phase, ready), level, diameter = 200.dp)
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
        } else if (ready && model.setup.keyboardSelected) {
            // The real thing: the same keyboard, in a field, as in any other app.
            Spacer(Modifier.height(16.dp))
            Field(typed, { typed = it }, label = "", placeholder = "Or tap here and use the Hams keyboard")
        }
        Spacer(Modifier.height(24.dp))
        if (last != null) {
            PrimaryButton("Start using Hams", onDone, Modifier.fillMaxWidth())
            Spacer(Modifier.height(4.dp))
            QuietButton("Try again", {
                since = System.currentTimeMillis()
                typed = ""
                TryField.clear()
            })
        } else {
            LinkButton("Skip for now", onDone, color = c.muted)
        }
        Spacer(Modifier.height(12.dp))
    }
}
