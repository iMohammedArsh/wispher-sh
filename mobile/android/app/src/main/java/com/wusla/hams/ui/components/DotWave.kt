package com.wusla.hams.ui.components

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.wusla.hams.ui.theme.Tokens
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.math.sqrt

/** What the dot wave is showing. */
enum class DotMode {
    /** Waiting: a quiet line of dots with a slow glow travelling along it. */
    Idle,
    /** The mic is open: the voice scrolls in from the right, one column per tick. */
    Live,
    /** Transcribing or polishing: a swell sweeps along the line. */
    Working,
    /** Can't be used right now: a faint, still line. */
    Off,
    /** Recording is on hold: a still, half-lit line. */
    Paused,
}

private const val MAX_COLUMNS = 96
private const val TICK_MS = 60L

/**
 * The voice as a dot matrix: a row of columns, each a stack of dots mirrored
 * around the middle line, taller the louder you were at that moment. Newest on
 * the right, so it reads like a tape of what you just said. It only draws.
 */
@Composable
fun DotWave(
    mode: DotMode,
    level: Float,
    modifier: Modifier = Modifier,
    tint: Color = Tokens.colors.text,
) {
    val latest by rememberUpdatedState(level)
    val ring = remember { FloatArray(MAX_COLUMNS) }
    var tick by remember { mutableIntStateOf(0) }

    LaunchedEffect(mode) {
        ring.fill(0f)
        if (mode != DotMode.Live) return@LaunchedEffect
        var smooth = 0f
        while (isActive) {
            // Speech sits low on a 0..1 scale: a square root lifts it so quiet talkers still move the dots.
            val target = (sqrt(latest.coerceIn(0f, 1f)) * 1.25f).coerceAtMost(1f)
            smooth += (target - smooth) * if (target > smooth) 0.75f else 0.35f
            System.arraycopy(ring, 1, ring, 0, ring.size - 1)
            ring[ring.size - 1] = smooth
            tick++
            delay(TICK_MS)
        }
    }

    val t = rememberInfiniteTransition(label = "dots")
    val phase by t.animateFloat(0f, 1f, infiniteRepeatable(tween(2200, easing = LinearEasing)), label = "phase")

    Canvas(modifier) {
        @Suppress("UNUSED_VARIABLE") val redraw = tick
        val gap = 10.dp.toPx()
        val radius = 2.dp.toPx()
        val columns = ((size.width - 2 * radius) / gap).toInt().coerceIn(1, MAX_COLUMNS)
        val left = (size.width - (columns - 1) * gap) / 2f
        val midY = size.height / 2f
        val rows = ((midY - radius) / gap).toInt().coerceAtLeast(0)

        for (i in 0 until columns) {
            val x = left + i * gap
            val amount: Float
            val glow: Float
            when (mode) {
                DotMode.Live -> {
                    amount = ring[ring.size - columns + i]
                    glow = 1f
                }
                DotMode.Working -> {
                    // A swell travelling left to right, wrapping around.
                    val d = ((i.toFloat() / columns) - phase + 1.5f) % 1f - 0.5f
                    amount = 0.55f * exp(-(d * d) / 0.012f)
                    glow = 0.5f + amount
                }
                DotMode.Idle -> {
                    amount = 0f
                    // A soft glow rolling along the middle line.
                    glow = 0.30f + 0.30f * (0.5f + 0.5f * sin(2 * PI.toFloat() * (i.toFloat() / columns * 1.5f - phase)))
                }
                DotMode.Off -> {
                    amount = 0f
                    glow = 0.18f
                }
                DotMode.Paused -> {
                    amount = 0f
                    glow = 0.45f
                }
            }
            val n = (amount * rows).roundToInt().coerceIn(0, rows)
            for (k in -n..n) {
                val fade = if (k == 0) 1f else 1f - 0.62f * (abs(k).toFloat() / (n + 1))
                drawCircle(tint.copy(alpha = (glow * fade).coerceIn(0f, 1f)), radius, Offset(x, midY + k * gap))
            }
        }
    }
}
