package com.wusla.hams.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.wusla.hams.ui.theme.Tokens
import kotlin.math.PI
import kotlin.math.sin

/** What the orb is doing. */
enum class OrbMood {
    /** Waiting: breathes slowly. */
    Idle,
    /** The mic is open: grows, ripples and the bars follow the voice. */
    Listening,
    /** Transcribing or polishing: a mint ring turns around it, the bars wave. */
    Working,
    /** Just wrote something: a short mint flash, then idle. */
    Done,
    /** Something went wrong: a small shake. */
    Error,
    /** Can't be used right now (setup missing, password field): grey and still. */
    Off,
}

// The Hams mark (ic_mark.xml, a 600-unit box): five bars that read as an H.
private val BAR_X = floatArrayOf(86f, 197f, 300f, 403f, 514f).map { it / 600f - 0.5f }
private val BAR_W = floatArrayOf(92f, 76f, 76f, 76f, 92f).map { it / 600f }
private val BAR_H = floatArrayOf(520f, 220f, 320f, 220f, 520f).map { it / 600f }
// While listening the bars become a waveform: tallest in the middle.
private val WAVE = floatArrayOf(0.55f, 0.85f, 1f, 0.85f, 0.55f)

/**
 * The voice orb: the one thing in Hams that is alive. Coral (what you say),
 * with the five-bar mark inside. Used big on Home and onboarding and as the
 * mic in the keyboard. `diameter` is the whole footprint, halo included; the
 * coral core is about 70% of it. It only draws: gestures stay with the caller.
 */
@Composable
fun Orb(
    mood: OrbMood,
    level: Float,
    modifier: Modifier = Modifier,
    diameter: Dp = 200.dp,
    command: Boolean = false,
) {
    val c = Tokens.colors
    val t = rememberInfiniteTransition(label = "orb")
    val breathe by t.animateFloat(0f, 1f, infiniteRepeatable(tween(2400, easing = FastOutSlowInEasing), RepeatMode.Reverse), label = "breathe")
    val ripple by t.animateFloat(0f, 1f, infiniteRepeatable(tween(1700, easing = LinearEasing)), label = "ripple")
    val spin by t.animateFloat(0f, 360f, infiniteRepeatable(tween(1800, easing = LinearEasing)), label = "spin")
    val clock by t.animateFloat(0f, (2 * PI).toFloat(), infiniteRepeatable(tween(1000, easing = LinearEasing)), label = "clock")

    val listening = mood == OrbMood.Listening
    val working = mood == OrbMood.Working
    val off = mood == OrbMood.Off
    val voice by animateFloatAsState(if (listening) level.coerceIn(0f, 1f) else 0f, tween(90), label = "level")
    val grow by animateFloatAsState(
        when (mood) {
            OrbMood.Listening -> 1.07f
            OrbMood.Working -> 0.95f
            else -> 1f
        },
        spring(dampingRatio = 0.55f, stiffness = 300f), label = "grow",
    )
    val wave by animateFloatAsState(if (listening) 1f else 0f, spring(dampingRatio = 0.8f, stiffness = 400f), label = "wave")
    val shimmer by animateFloatAsState(if (working) 1f else 0f, tween(300), label = "shimmer")
    val dim by animateFloatAsState(if (off) 1f else 0f, tween(300), label = "off")

    val flash = remember { Animatable(0f) }
    val shake = remember { Animatable(0f) }
    LaunchedEffect(mood) {
        when (mood) {
            OrbMood.Done -> {
                flash.snapTo(1f)
                flash.animateTo(0f, tween(900, easing = FastOutSlowInEasing))
            }
            OrbMood.Error -> {
                for (x in listOf(10f, -8f, 6f, -4f, 0f)) shake.animateTo(x, tween(60))
            }
            else -> Unit
        }
    }

    // Coral core, forest when it edits a selection (command), grey when off.
    val (light, mid, deep) = when {
        command && c.dark -> Triple(Color(0xFFC4F2D6), c.mint, Color(0xFF4DB884))
        command -> Triple(Color(0xFF2F7356), c.primary, Color(0xFF082619))
        else -> Triple(c.voiceLight, c.voice, c.voiceDeep)
    }
    val grey = c.control
    val mint = c.mint

    Canvas(modifier.size(diameter).graphicsLayer { translationX = shake.value }) {
        val center = Offset(size.width / 2, size.height / 2)
        val r = size.minDimension / 2 * 0.70f
        val f = flash.value
        fun tone(color: Color) = lerp(lerp(color, mint, f * 0.85f), grey, dim)
        val coreMid = tone(mid)

        // Halo: breathes when idle, swells with the voice.
        val haloR = r * (1.28f + 0.06f * breathe * (1 - wave) + 0.22f * voice)
        drawCircle(
            Brush.radialGradient(listOf(coreMid.copy(alpha = 0.30f * (1 - dim * 0.7f)), coreMid.copy(alpha = 0f)), center, haloR),
            radius = haloR, center = center,
        )

        // Ripples while listening.
        if (wave > 0.01f) {
            for (i in 0..1) {
                val p = (ripple + i * 0.5f) % 1f
                val rr = r * (1.02f + p * (0.30f + voice * 0.20f))
                drawCircle(
                    coreMid.copy(alpha = (1 - p) * 0.45f * wave),
                    radius = rr, center = center, style = Stroke(width = (1.5f + 2.5f * voice).dp.toPx()),
                )
            }
        }

        // A mint ring turning while Hams works.
        if (shimmer > 0.01f) {
            rotate(spin, center) {
                drawCircle(
                    Brush.sweepGradient(listOf(mint.copy(alpha = 0f), mint.copy(alpha = 0.9f * shimmer), mint.copy(alpha = 0f)), center),
                    radius = r * 1.12f, center = center, style = Stroke(width = 3.dp.toPx()),
                )
            }
        }

        // The core: lit from the top left.
        val scale = grow * (1f + 0.035f * breathe * (1 - wave) * (1 - shimmer) + 0.07f * voice)
        val cr = r * scale
        drawCircle(
            Brush.radialGradient(listOf(tone(light), coreMid, tone(deep)), center + Offset(-cr * 0.35f, -cr * 0.45f), cr * 1.7f),
            radius = cr, center = center,
        )
        drawCircle(
            Brush.radialGradient(listOf(Color.White.copy(alpha = 0.28f), Color.White.copy(alpha = 0f)), center + Offset(-cr * 0.32f, -cr * 0.42f), cr * 0.75f),
            radius = cr, center = center,
        )

        // The bars: the mark at rest, a waveform while listening, a wave while working.
        val box = cr * 1.0f
        val ink = lerp(Color.White, c.surface, dim).copy(alpha = if (off) 0.9f else 0.96f)
        for (i in 0 until 5) {
            val markH = BAR_H[i] * box
            val waveH = box * (0.16f + 0.62f * voice * WAVE[i] + 0.05f * sin(clock * 2 + i * 1.3f))
            var h = markH + (waveH - markH) * wave
            h *= 1f - shimmer * 0.35f * (0.5f + 0.5f * sin(clock + i * 0.9f))
            val w = BAR_W[i] * box * 0.92f
            val x = center.x + BAR_X[i] * box * 1.0f - w / 2
            drawRoundRect(ink, Offset(x, center.y - h / 2), Size(w, h), CornerRadius(w / 2, w / 2))
        }
    }
}
