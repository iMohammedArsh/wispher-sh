package com.wusla.hams.ime

import android.text.InputType
import android.view.inputmethod.EditorInfo
import com.wusla.hams.core.FieldKind
import org.junit.Assert.assertEquals
import org.junit.Test

class FieldKindsTest {
    private val text = InputType.TYPE_CLASS_TEXT

    @Test
    fun passwordsAreRecognisedInEveryForm() {
        listOf(
            text or InputType.TYPE_TEXT_VARIATION_PASSWORD,
            text or InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
            text or InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
            InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_VARIATION_PASSWORD,
        ).forEach { assertEquals(FieldKind.PASSWORD, FieldKinds.kindOf(it)) }
    }

    @Test
    fun addressesAndNumbers() {
        assertEquals(FieldKind.EMAIL, FieldKinds.kindOf(text or InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS))
        assertEquals(FieldKind.EMAIL, FieldKinds.kindOf(text or InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS))
        assertEquals(FieldKind.URL, FieldKinds.kindOf(text or InputType.TYPE_TEXT_VARIATION_URI))
        assertEquals(FieldKind.PHONE, FieldKinds.kindOf(InputType.TYPE_CLASS_PHONE))
        assertEquals(FieldKind.NUMBER, FieldKinds.kindOf(InputType.TYPE_CLASS_NUMBER))
    }

    @Test
    fun chatBoxesAreShortTextAndNotesAreText() {
        assertEquals(FieldKind.SHORT_TEXT, FieldKinds.kindOf(text or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES))
        assertEquals(FieldKind.TEXT, FieldKinds.kindOf(text or InputType.TYPE_TEXT_FLAG_MULTI_LINE))
        assertEquals(FieldKind.TEXT, FieldKinds.kindOf(InputType.TYPE_NULL))
    }

    @Test
    fun enterSendsInChatsAndBreaksLinesInNotes() {
        assertEquals(FieldKinds.Enter.SEND, FieldKinds.enterOf(text, EditorInfo.IME_ACTION_SEND))
        assertEquals(FieldKinds.Enter.SEARCH, FieldKinds.enterOf(text, EditorInfo.IME_ACTION_SEARCH))
        assertEquals(FieldKinds.Enter.NEWLINE, FieldKinds.enterOf(text or InputType.TYPE_TEXT_FLAG_MULTI_LINE, EditorInfo.IME_ACTION_SEND))
        assertEquals(FieldKinds.Enter.NEWLINE, FieldKinds.enterOf(text, EditorInfo.IME_ACTION_SEND or EditorInfo.IME_FLAG_NO_ENTER_ACTION))
        assertEquals(FieldKinds.Enter.NEWLINE, FieldKinds.enterOf(text, EditorInfo.IME_ACTION_UNSPECIFIED))
    }
}
