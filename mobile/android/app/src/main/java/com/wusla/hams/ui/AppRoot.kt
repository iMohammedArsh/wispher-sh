package com.wusla.hams.ui

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.database.ContentObserver
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.view.inputmethod.InputMethodManager
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.layout.width
import com.wusla.hams.R
import com.wusla.hams.ui.components.HamsIcon
import com.wusla.hams.ui.components.bounceClick
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.core.app.ActivityCompat
import androidx.lifecycle.compose.LifecycleResumeEffect
import com.wusla.hams.core.Downloads
import com.wusla.hams.core.Hams
import com.wusla.hams.ui.components.Mark
import com.wusla.hams.ui.components.Muted
import com.wusla.hams.ui.components.Panel
import com.wusla.hams.ui.components.PrimaryButton
import com.wusla.hams.ui.components.QuietButton
import com.wusla.hams.ui.components.ScreenTitle
import com.wusla.hams.ui.screens.HomeScreen
import com.wusla.hams.ui.screens.OnboardingScreen
import com.wusla.hams.ui.screens.SettingsScreen
import com.wusla.hams.ui.screens.WordsScreen
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens
import kotlinx.coroutines.delay

enum class Tab(val label: String, val icon: Int) { Home("Home", R.drawable.ic_home), You("You", R.drawable.ic_person) }

@Composable
fun AppRoot(section: String?, onSectionHandled: () -> Unit, requestMic: ((Boolean) -> Unit) -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val model = remember { AppModel(context.applicationContext, scope) }
    val dataVersion by Hams.dataVersion.collectAsState()
    val ready by Hams.ready.collectAsState()
    var tab by rememberSaveable { mutableStateOf(Tab.Home) }
    var settingsArea by rememberSaveable { mutableStateOf<String?>(null) }

    // Also when a download ends: the screen may have been recreated while it ran.
    LaunchedEffect(dataVersion, ready, Downloads.finished) { if (ready) model.reload() }
    LifecycleResumeEffect(ready) {
        model.refreshSetup()
        onPauseOrDispose { }
    }
    // Choosing the keyboard in Android's picker is a system dialog: this activity isn't paused, so the
    // resume check above never sees it. Watch the settings themselves, so setup steps notice at once.
    DisposableEffect(context) {
        val resolver = context.contentResolver
        val observer = object : ContentObserver(Handler(Looper.getMainLooper())) {
            override fun onChange(selfChange: Boolean) = model.refreshSetup()
        }
        listOf(Settings.Secure.DEFAULT_INPUT_METHOD, Settings.Secure.ENABLED_INPUT_METHODS).forEach {
            resolver.registerContentObserver(Settings.Secure.getUriFor(it), false, observer)
        }
        onDispose { resolver.unregisterContentObserver(observer) }
    }
    // Opened from the keyboard: go where the user needs to be.
    LaunchedEffect(section) {
        when (section) {
            "permission" -> requestMic { model.refreshSetup() }
            "model" -> { tab = Tab.You; settingsArea = "model" }
            "languages" -> { tab = Tab.You; settingsArea = "languages" }
            "settings" -> tab = Tab.You
        }
        if (section != null) onSectionHandled()
    }
    // Android back: out of a settings area first, then back to Home.
    BackHandler(enabled = settingsArea != null || tab != Tab.Home) {
        if (settingsArea != null) settingsArea = null else tab = Tab.Home
    }

    val c = Tokens.colors
    Box(Modifier.fillMaxSize().background(c.ground)) {
        val engineError = Hams.startError
        val settings = model.settings
        when {
            !ready || (Hams.engineOrNull != null && settings == null) -> Starting()
            Hams.engineOrNull == null -> EngineError(engineError)
            settings == null -> Unit
            !settings.onboardingDone -> OnboardingScreen(model, requestMic) {
                model.save { edit { put("onboarding_done", true) } }
            }
            else -> {
                Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))) {
                    AnimatedContent(
                        tab to settingsArea,
                        transitionSpec = { (fadeIn(tween(220)) + slideInVertically(tween(260)) { it / 40 }) togetherWith fadeOut(tween(120)) },
                        label = "screen",
                    ) { (current, area) ->
                        when (current) {
                            Tab.Home -> HomeScreen(model, requestMic) { a -> tab = Tab.You; settingsArea = a }
                            Tab.You -> if (area == "words") {
                                WordsScreen(model) { settingsArea = null }
                            } else {
                                SettingsScreen(model, area, requestMic) { settingsArea = it }
                            }
                        }
                    }
                }
                TabBar(tab, model.suggestions.size, Modifier.align(Alignment.BottomCenter)) {
                    if (it == tab) settingsArea = null
                    tab = it
                }
            }
        }
        Toast(model.message, Modifier.align(Alignment.BottomCenter)) { model.message = null }
        DownloadDialog(model)
    }
}

/** Before a big download: on mobile data, ask; without the room for it, say so. */
@Composable
private fun DownloadDialog(model: AppModel) {
    val pending = model.pendingDownload ?: return
    val c = Tokens.colors
    Dialog(onDismissRequest = model::dismissDownload) {
        Panel {
            val noSpace = pending.check == DownloadCheck.NoSpace
            Text(if (noSpace) "Not enough space" else "Download on mobile data?", style = HamsType.heading, color = c.text)
            Spacer(Modifier.height(8.dp))
            Muted(
                if (noSpace) "${pending.label} needs about ${pending.sizeMb} MB of free space. Free some up, then try again."
                else "${pending.label} is ${pending.sizeMb} MB and you're on a metered connection (mobile data or a hotspot). It's downloaded once, and Hams carries on from where it stopped if it's interrupted.",
            )
            Spacer(Modifier.height(20.dp))
            if (noSpace) {
                PrimaryButton("OK", model::dismissDownload, Modifier.fillMaxWidth())
            } else {
                PrimaryButton("Download anyway", model::confirmDownload, Modifier.fillMaxWidth())
                Spacer(Modifier.height(4.dp))
                QuietButton("Wait for Wi-Fi", model::dismissDownload, Modifier.fillMaxWidth())
            }
        }
    }
}

/** A floating forest pill with the two places in the app. */
@Composable
private fun TabBar(current: Tab, pendingSuggestions: Int, modifier: Modifier, onSelect: (Tab) -> Unit) {
    val c = Tokens.colors
    // Forest on paper by day; a lifted night-green by night.
    val bar = if (c.dark) c.active else c.primary
    val ink = if (c.dark) c.text else c.primaryInk
    Row(
        modifier
            .windowInsetsPadding(WindowInsets.navigationBars)
            .padding(bottom = 14.dp)
            .shadow(16.dp, RoundedCornerShape(50), ambientColor = Color.Black.copy(alpha = 0.3f), spotColor = Color.Black.copy(alpha = 0.3f))
            .clip(RoundedCornerShape(50))
            .background(bar)
            .padding(6.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Tab.entries.forEach { tab ->
            val selected = tab == current
            val fill by animateColorAsState(if (selected) ink.copy(alpha = 0.16f) else bar, label = "tab")
            Row(
                Modifier
                    .clip(RoundedCornerShape(50))
                    .background(fill)
                    .bounceClick { onSelect(tab) }
                    .padding(horizontal = 22.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box {
                    HamsIcon(tab.icon, tint = if (selected) ink else ink.copy(alpha = 0.6f), size = 22.dp)
                    if (tab == Tab.You && pendingSuggestions > 0) {
                        Box(Modifier.align(Alignment.TopEnd).size(8.dp).clip(CircleShape).background(c.voice))
                    }
                }
                Spacer(Modifier.width(8.dp))
                Text(tab.label, style = HamsType.bodyStrong, color = if (selected) ink else ink.copy(alpha = 0.6f))
            }
        }
    }
}

/** A floating message (floats above the tab bar). */
@Composable
private fun Toast(message: String?, modifier: Modifier, onDismiss: () -> Unit) {
    if (message == null) return
    LaunchedEffect(message) {
        delay(3_500)
        onDismiss()
    }
    val c = Tokens.colors
    Box(
        modifier
            .windowInsetsPadding(WindowInsets.navigationBars)
            .padding(bottom = 96.dp, start = 24.dp, end = 24.dp)
            .shadow(10.dp, RoundedCornerShape(18.dp))
            .clip(RoundedCornerShape(18.dp))
            .background(if (c.dark) c.active else c.primary)
            .clickable(onClick = onDismiss)
            .padding(horizontal = 18.dp, vertical = 14.dp),
    ) { Text(message, style = HamsType.bodyStrong, color = if (c.dark) c.text else c.primaryInk) }
}

/** While the engine loads on a cold start: the orb, breathing. */
@Composable
private fun Starting() {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        com.wusla.hams.ui.components.Orb(com.wusla.hams.ui.components.OrbMood.Idle, 0f, diameter = 200.dp)
    }
}

@Composable
private fun EngineError(error: String?) {
    Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).padding(24.dp)) {
        Mark(size = 32)
        Spacer(Modifier.height(16.dp))
        ScreenTitle("Hams couldn't start", "This build is missing its speech engine or couldn't open its storage.")
        Panel { Muted(error ?: "Unknown error") }
    }
}

// ---- system screens the app sends people to ----------------------------------------------------

fun openKeyboardSettings(context: Context) {
    context.startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
}

fun showKeyboardPicker(context: Context) {
    context.getSystemService(InputMethodManager::class.java)?.showInputMethodPicker()
}

/**
 * Whether Android would still show the microphone prompt. Only meaningful right
 * after the user refused it (before the first prompt it is also false); see
 * [micAskAfterRefusal].
 */
fun micPromptCanBeShownAgain(context: Context): Boolean {
    var current: Context? = context
    while (current is ContextWrapper) {
        if (current is Activity) return ActivityCompat.shouldShowRequestPermissionRationale(current, Manifest.permission.RECORD_AUDIO)
        current = current.baseContext
    }
    return false
}

fun openAppSettings(context: Context) {
    context.startActivity(
        Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
    )
}
