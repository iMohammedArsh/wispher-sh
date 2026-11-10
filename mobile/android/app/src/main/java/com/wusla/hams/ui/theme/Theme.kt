package com.wusla.hams.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.ExperimentalTextApi
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.wusla.hams.R

/**
 * The landing page's colours (site/src/site.css), where every colour is a stage
 * of one dictation:
 *  - voice (coral): what you say. The orb and the live mic.
 *  - forest: Hams at work, and the primary action.
 *  - green / mint: what was written. Changed words, "on" states.
 *  - paper: the page your text lands on.
 * Dark mode is "forest night": the same stages on a deep green ground.
 */
@Immutable
data class HamsColors(
    val ground: Color,
    val surface: Color,
    /** A slightly lifted surface (cards on cards, inputs). */
    val raised: Color,
    /** One step from the ground: tonal buttons, chips, keys, tracks. */
    val active: Color,
    val border: Color,
    val rule: Color,
    val control: Color,
    val text: Color,
    val muted: Color,
    val faint: Color,
    val primary: Color,
    val primaryInk: Color,
    val data: Color,
    val dataSoft: Color,
    val mint: Color,
    val voice: Color,
    val voiceDeep: Color,
    val voiceLight: Color,
    val voiceSoft: Color,
    val voiceInk: Color,
    val live: Color,
    val liveSoft: Color,
    val negative: Color,
    val warningBg: Color,
    val warningInk: Color,
    val dark: Boolean,
)

val LightColors = HamsColors(
    ground = Color(0xFFEDF5EF), surface = Color(0xFFFFFFFF), raised = Color(0xFFF5FAF6), active = Color(0xFFDBEAE0),
    border = Color(0xFFD3E3D8), rule = Color(0xFFE4EEE7), control = Color(0xFFB9D0C1),
    text = Color(0xFF0B2219), muted = Color(0xFF42564A), faint = Color(0xFF6B8073),
    primary = Color(0xFF0F3D2B), primaryInk = Color(0xFFFFFFFF),
    data = Color(0xFF1F7A50), dataSoft = Color(0xFFCDEEDB), mint = Color(0xFF8EE3B1),
    voice = Color(0xFFFF6A4D), voiceDeep = Color(0xFFE94F33), voiceLight = Color(0xFFFFA08A),
    voiceSoft = Color(0xFFFFE4DC), voiceInk = Color(0xFF0B2219),
    live = Color(0xFFD63A3F), liveSoft = Color(0xFFFFE6E0),
    negative = Color(0xFFB4232A), warningBg = Color(0xFFFFF4E0), warningInk = Color(0xFF8A4200),
    dark = false,
)

val DarkColors = HamsColors(
    ground = Color(0xFF08170F), surface = Color(0xFF0F241A), raised = Color(0xFF132B20), active = Color(0xFF1A3729),
    border = Color(0xFF1E3A2C), rule = Color(0xFF173024), control = Color(0xFF2B4A3A),
    text = Color(0xFFEAF6EE), muted = Color(0xFFA9CBB6), faint = Color(0xFF7E9C8A),
    primary = Color(0xFF8EE3B1), primaryInk = Color(0xFF08170F),
    data = Color(0xFF6FDCA2), dataSoft = Color(0xFF1B4431), mint = Color(0xFF8EE3B1),
    voice = Color(0xFFFF8166), voiceDeep = Color(0xFFFF6A4D), voiceLight = Color(0xFFFFB3A1),
    voiceSoft = Color(0xFF3B2019), voiceInk = Color(0xFF08170F),
    live = Color(0xFFFF6B6F), liveSoft = Color(0xFF3A1A17),
    negative = Color(0xFFFF8A85), warningBg = Color(0xFF2E2510), warningInk = Color(0xFFF3C766),
    dark = true,
)

@OptIn(ExperimentalTextApi::class)
private fun hanken(weight: Int) = Font(
    R.font.hanken_grotesk, FontWeight(weight),
    variationSettings = FontVariation.Settings(FontVariation.weight(weight)),
)

@OptIn(ExperimentalTextApi::class)
private fun fraunces(weight: Int) = Font(
    R.font.fraunces, FontWeight(weight),
    variationSettings = FontVariation.Settings(
        FontVariation.weight(weight),
        FontVariation.Setting("opsz", 72f),
        FontVariation.Setting("SOFT", 100f),
    ),
)

/** Hanken Grotesk for everything you read and tap. */
val TextFont = FontFamily(hanken(400), hanken(500), hanken(600))

/** Fraunces (soft, light) for the few big lines that set the mood. */
val DisplayFont = FontFamily(fraunces(300), fraunces(400))

/** Type scale 13/15/17/20/24 + display 30/40. */
object HamsType {
    val small = TextStyle(fontFamily = TextFont, fontSize = 13.sp, lineHeight = 18.sp)
    val label = TextStyle(fontFamily = TextFont, fontSize = 14.sp, lineHeight = 20.sp, fontWeight = FontWeight.Medium)
    val body = TextStyle(fontFamily = TextFont, fontSize = 15.sp, lineHeight = 22.sp)
    val bodyStrong = TextStyle(fontFamily = TextFont, fontSize = 15.sp, lineHeight = 22.sp, fontWeight = FontWeight.Medium)
    val written = TextStyle(fontFamily = TextFont, fontSize = 17.sp, lineHeight = 25.sp)
    val section = TextStyle(fontFamily = TextFont, fontSize = 17.sp, lineHeight = 24.sp, fontWeight = FontWeight.SemiBold)
    val heading = TextStyle(fontFamily = TextFont, fontSize = 20.sp, lineHeight = 26.sp, fontWeight = FontWeight.SemiBold)
    val title = TextStyle(fontFamily = DisplayFont, fontSize = 30.sp, lineHeight = 36.sp, fontWeight = FontWeight.Light, letterSpacing = (-0.3).sp)
    val display = TextStyle(fontFamily = DisplayFont, fontSize = 40.sp, lineHeight = 44.sp, fontWeight = FontWeight.Light, letterSpacing = (-0.6).sp)
    val number = TextStyle(fontFamily = DisplayFont, fontSize = 28.sp, lineHeight = 32.sp, fontWeight = FontWeight.Normal)
}

object HamsShape {
    val card = 24.dp
    val control = 16.dp
    val key = 12.dp
    val pad = 20.dp
    val gap = 16.dp
}

val LocalHamsColors = staticCompositionLocalOf { LightColors }

object Tokens {
    val colors: HamsColors
        @Composable get() = LocalHamsColors.current
}

@Composable
fun HamsTheme(dark: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    val colors = if (dark) DarkColors else LightColors
    // Material components (menus, text fields, switches) pick up the same palette and font.
    val scheme = (if (dark) darkColorScheme() else lightColorScheme()).copy(
        primary = colors.primary, onPrimary = colors.primaryInk, background = colors.ground, onBackground = colors.text,
        surface = colors.surface, onSurface = colors.text, surfaceVariant = colors.raised, onSurfaceVariant = colors.muted,
        outline = colors.control, outlineVariant = colors.border, error = colors.negative, secondary = colors.data,
        surfaceContainer = colors.surface, surfaceContainerHigh = colors.raised, surfaceContainerHighest = colors.active,
        surfaceContainerLow = colors.surface, surfaceContainerLowest = colors.surface,
    )
    val base = Typography()
    val typography = Typography(
        bodyLarge = base.bodyLarge.copy(fontFamily = TextFont),
        bodyMedium = base.bodyMedium.copy(fontFamily = TextFont),
        bodySmall = base.bodySmall.copy(fontFamily = TextFont),
        labelLarge = base.labelLarge.copy(fontFamily = TextFont),
        labelMedium = base.labelMedium.copy(fontFamily = TextFont),
        labelSmall = base.labelSmall.copy(fontFamily = TextFont),
        titleMedium = base.titleMedium.copy(fontFamily = TextFont),
    )
    CompositionLocalProvider(LocalHamsColors provides colors) {
        MaterialTheme(colorScheme = scheme, typography = typography, content = content)
    }
}
