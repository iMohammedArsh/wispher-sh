package com.wusla.hams.ui.components

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Icon
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.wusla.hams.R
import com.wusla.hams.ui.theme.HamsShape
import com.wusla.hams.ui.theme.HamsType
import com.wusla.hams.ui.theme.Tokens
import kotlinx.coroutines.delay

/** Clickable that shrinks a little under the finger (no ripple): everything tappable feels soft. */
fun Modifier.bounceClick(enabled: Boolean = true, onClick: () -> Unit): Modifier = composed {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    val scale by animateFloatAsState(
        if (pressed && enabled) 0.96f else 1f,
        spring(dampingRatio = 0.6f, stiffness = Spring.StiffnessMedium), label = "press",
    )
    this
        .graphicsLayer { scaleX = scale; scaleY = scale }
        .clickable(interactionSource = source, indication = null, enabled = enabled, onClick = onClick)
}

/** A card: white (or night green) on the paper ground, big soft corners, no border. */
@Composable
fun Panel(
    modifier: Modifier = Modifier,
    padding: PaddingValues = PaddingValues(20.dp),
    color: Color = Tokens.colors.surface,
    content: @Composable ColumnScope.() -> Unit,
) {
    Column(
        modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(HamsShape.card))
            .background(color)
            .padding(padding),
        content = content,
    )
}

@Composable
fun ScreenTitle(title: String, subtitle: String? = null, modifier: Modifier = Modifier) {
    Column(modifier.padding(top = 8.dp, bottom = 20.dp)) {
        Text(title, style = HamsType.title, color = Tokens.colors.text)
        if (subtitle != null) {
            Spacer(Modifier.height(8.dp))
            Text(subtitle, style = HamsType.body, color = Tokens.colors.muted)
        }
    }
}

@Composable
fun SectionTitle(text: String, modifier: Modifier = Modifier) {
    Text(text, style = HamsType.label, color = Tokens.colors.muted, modifier = modifier.padding(start = 4.dp, top = 24.dp, bottom = 10.dp))
}

@Composable
fun Muted(text: String, modifier: Modifier = Modifier) {
    Text(text, style = HamsType.body, color = Tokens.colors.muted, modifier = modifier)
}

@Composable
fun HamsIcon(@DrawableRes id: Int, modifier: Modifier = Modifier, tint: Color = Tokens.colors.text, size: Dp = 22.dp) {
    Icon(painterResource(id), contentDescription = null, tint = tint, modifier = modifier.size(size))
}

@Composable
private fun SolidButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier,
    enabled: Boolean,
    container: Color,
    ink: Color,
    @DrawableRes icon: Int?,
    height: Dp = 52.dp,
) {
    val c = Tokens.colors
    Row(
        modifier
            .heightIn(min = height)
            .clip(RoundedCornerShape(HamsShape.control))
            .background(if (enabled) container else c.active)
            .bounceClick(enabled, onClick)
            .padding(horizontal = 22.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            HamsIcon(icon, tint = if (enabled) ink else c.faint, size = 20.dp)
            Spacer(Modifier.width(8.dp))
        }
        Text(text, style = HamsType.bodyStrong.copy(fontSize = HamsType.written.fontSize), color = if (enabled) ink else c.faint, textAlign = TextAlign.Center)
    }
}

/** The forest primary button. */
@Composable
fun PrimaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, @DrawableRes icon: Int? = null) {
    val c = Tokens.colors
    SolidButton(text, onClick, modifier, enabled, c.primary, c.primaryInk, icon)
}

/** Coral: only for actions that start talking. */
@Composable
fun VoiceButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, @DrawableRes icon: Int? = null) {
    val c = Tokens.colors
    SolidButton(text, onClick, modifier, enabled, c.voice, c.voiceInk, icon)
}

/** A tonal button, one step from the ground. */
@Composable
fun QuietButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, @DrawableRes icon: Int? = null) {
    val c = Tokens.colors
    Row(
        modifier
            .heightIn(min = 44.dp)
            .clip(RoundedCornerShape(14.dp))
            .background(c.active)
            .bounceClick(enabled, onClick)
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
    ) {
        if (icon != null) {
            HamsIcon(icon, tint = if (enabled) c.text else c.faint, size = 18.dp)
            Spacer(Modifier.width(6.dp))
        }
        Text(text, style = HamsType.label, color = if (enabled) c.text else c.faint)
    }
}

@Composable
fun LinkButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, color: Color = Tokens.colors.text) {
    Text(
        text, style = HamsType.label, color = color,
        modifier = modifier.clip(RoundedCornerShape(10.dp)).bounceClick(onClick = onClick).padding(horizontal = 10.dp, vertical = 8.dp),
    )
}

/** A round tonal icon button. */
@Composable
fun RoundIcon(@DrawableRes icon: Int, onClick: () -> Unit, modifier: Modifier = Modifier, description: String? = null, size: Dp = 44.dp, container: Color = Tokens.colors.active, tint: Color = Tokens.colors.text) {
    Box(
        modifier.size(size).clip(CircleShape).background(container).bounceClick(onClick = onClick)
            .then(if (description != null) Modifier.semantics { contentDescription = description } else Modifier),
        contentAlignment = Alignment.Center,
    ) { HamsIcon(icon, tint = tint, size = size * 0.48f) }
}

/** A pill chip; `selected` fills it with the primary colour. */
@Composable
fun Chip(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, selected: Boolean = false, enabled: Boolean = true) {
    val c = Tokens.colors
    Box(
        modifier
            .clip(RoundedCornerShape(50))
            .background(if (selected) c.primary else c.active)
            .bounceClick(enabled, onClick)
            .padding(horizontal = 14.dp, vertical = 8.dp),
    ) {
        Text(
            text, style = HamsType.label,
            color = when {
                !enabled -> c.faint
                selected -> c.primaryInk
                else -> c.text
            },
        )
    }
}

enum class Tone { Positive, Warning, Negative, Neutral }

@Composable
fun Pill(text: String, tone: Tone = Tone.Neutral, modifier: Modifier = Modifier) {
    val c = Tokens.colors
    val (bg, ink) = when (tone) {
        Tone.Positive -> c.dataSoft to c.data
        Tone.Warning -> c.warningBg to c.warningInk
        Tone.Negative -> c.liveSoft to c.negative
        Tone.Neutral -> c.active to c.muted
    }
    Text(
        text, style = HamsType.small, color = ink,
        modifier = modifier.clip(RoundedCornerShape(50)).background(bg).padding(horizontal = 10.dp, vertical = 3.dp),
    )
}

/** A setting with a switch. "On" is the written-green. */
@Composable
fun SwitchRow(title: String, subtitle: String?, checked: Boolean, onChange: (Boolean) -> Unit, enabled: Boolean = true) {
    val c = Tokens.colors
    Row(
        Modifier
            .fillMaxWidth()
            .clickable(enabled = enabled, indication = null, interactionSource = remember { MutableInteractionSource() }) { onChange(!checked) }
            .padding(vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = HamsType.written.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Medium), color = if (enabled) c.text else c.faint)
            if (subtitle != null) {
                Spacer(Modifier.height(2.dp))
                Text(subtitle, style = HamsType.small, color = c.muted)
            }
        }
        Spacer(Modifier.width(14.dp))
        Switch(
            checked = checked, onCheckedChange = onChange, enabled = enabled,
            colors = SwitchDefaults.colors(
                checkedTrackColor = c.data, checkedThumbColor = c.surface, checkedBorderColor = c.data,
                uncheckedTrackColor = c.active, uncheckedThumbColor = c.surface, uncheckedBorderColor = c.active,
            ),
        )
    }
}

/** A row that opens something: optional icon tile, title, a value on the right, chevron. */
@Composable
fun ValueRow(title: String, value: String, onClick: () -> Unit, subtitle: String? = null, @DrawableRes icon: Int? = null, badge: Int = 0) {
    val c = Tokens.colors
    Row(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).bounceClick(onClick = onClick).padding(vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            Box(Modifier.size(40.dp).clip(RoundedCornerShape(12.dp)).background(c.active), contentAlignment = Alignment.Center) {
                HamsIcon(icon, tint = c.primary.takeIf { !c.dark } ?: c.mint, size = 20.dp)
            }
            Spacer(Modifier.width(14.dp))
        }
        Column(Modifier.weight(1f)) {
            Text(title, style = HamsType.written.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.Medium), color = c.text)
            if (subtitle != null) Text(subtitle, style = HamsType.small, color = c.muted)
        }
        if (badge > 0) {
            Text(
                "$badge", style = HamsType.small.copy(fontWeight = androidx.compose.ui.text.font.FontWeight.SemiBold), color = c.voiceInk,
                modifier = Modifier.clip(RoundedCornerShape(50)).background(c.voice).padding(horizontal = 8.dp, vertical = 2.dp),
            )
            Spacer(Modifier.width(8.dp))
        }
        if (value.isNotEmpty()) {
            Text(value, style = HamsType.body, color = c.muted, maxLines = 1)
            Spacer(Modifier.width(4.dp))
        }
        HamsIcon(R.drawable.ic_chevron, tint = c.faint, size = 18.dp)
    }
}

@Composable
fun Rule(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().height(1.dp).background(Tokens.colors.rule))
}

/** A filled, rounded text field without the underline. */
@Composable
fun Field(
    value: String,
    onChange: (String) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    secret: Boolean = false,
    singleLine: Boolean = true,
    keyboard: KeyboardType = KeyboardType.Text,
    placeholder: String? = null,
) {
    val c = Tokens.colors
    TextField(
        value = value, onValueChange = onChange, modifier = modifier.fillMaxWidth(),
        label = if (label.isEmpty()) null else ({ Text(label, style = HamsType.small) }),
        placeholder = placeholder?.let { { Text(it, style = HamsType.body, color = c.faint) } },
        singleLine = singleLine, textStyle = HamsType.written.copy(color = c.text),
        visualTransformation = if (secret) PasswordVisualTransformation() else VisualTransformation.None,
        keyboardOptions = KeyboardOptions(keyboardType = if (secret) KeyboardType.Password else keyboard),
        shape = RoundedCornerShape(HamsShape.control),
        colors = TextFieldDefaults.colors(
            focusedContainerColor = c.active, unfocusedContainerColor = c.active, disabledContainerColor = c.active,
            focusedIndicatorColor = Color.Transparent, unfocusedIndicatorColor = Color.Transparent, disabledIndicatorColor = Color.Transparent,
            focusedLabelColor = c.muted, unfocusedLabelColor = c.muted, cursorColor = c.text,
            focusedTextColor = c.text, unfocusedTextColor = c.text,
        ),
    )
}

/** The Hams glyph (five voice bars that read as an H). */
@Composable
fun Mark(modifier: Modifier = Modifier, tint: Color = Tokens.colors.voice, size: Int = 20) {
    Icon(painterResource(R.drawable.ic_mark), contentDescription = null, tint = tint, modifier = modifier.size(size.dp))
}

/** A numbered step dot for setup checklists. */
@Composable
fun StepDot(number: Int, done: Boolean) {
    val c = Tokens.colors
    Box(
        Modifier.size(26.dp).clip(CircleShape).background(if (done) c.data else c.active),
        contentAlignment = Alignment.Center,
    ) {
        Text(if (done) "✓" else "$number", style = HamsType.small, color = if (done) c.surface else c.muted)
    }
}

@Composable
fun Spaced(content: @Composable ColumnScope.() -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp), content = content)
}

/** A screen header with a back arrow (sub-screens of You). */
@Composable
fun BackBar(onBack: () -> Unit, label: String = "Back") {
    Row(
        Modifier.clip(RoundedCornerShape(14.dp)).bounceClick(onClick = onBack).padding(vertical = 8.dp, horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        HamsIcon(R.drawable.ic_back, tint = Tokens.colors.text, size = 20.dp)
        Spacer(Modifier.width(6.dp))
        Text(label, style = HamsType.label, color = Tokens.colors.text)
    }
}

/**
 * A destructive link that asks for a second tap instead of a dialog (the
 * desktop app's ConfirmButton). Resets after a few seconds.
 */
@Composable
fun ConfirmLink(text: String, confirmText: String, onConfirm: () -> Unit) {
    var armed by remember { mutableStateOf(false) }
    LaunchedEffect(armed) {
        if (armed) {
            delay(3_000)
            armed = false
        }
    }
    LinkButton(if (armed) confirmText else text, {
        if (armed) {
            armed = false
            onConfirm()
        } else {
            armed = true
        }
    }, color = Tokens.colors.negative)
}
