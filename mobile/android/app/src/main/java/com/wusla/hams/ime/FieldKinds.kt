package com.wusla.hams.ime

import android.text.InputType
import android.view.inputmethod.EditorInfo
import com.wusla.hams.core.FieldKind

/** Pure mappings from Android's EditorInfo to what Hams needs (unit-tested). */
object FieldKinds {
    fun kindOf(inputType: Int): FieldKind {
        val variation = inputType and InputType.TYPE_MASK_VARIATION
        return when (inputType and InputType.TYPE_MASK_CLASS) {
            InputType.TYPE_CLASS_NUMBER ->
                if (variation == InputType.TYPE_NUMBER_VARIATION_PASSWORD) FieldKind.PASSWORD else FieldKind.NUMBER
            InputType.TYPE_CLASS_PHONE -> FieldKind.PHONE
            InputType.TYPE_CLASS_DATETIME -> FieldKind.NUMBER
            InputType.TYPE_CLASS_TEXT -> when (variation) {
                InputType.TYPE_TEXT_VARIATION_PASSWORD,
                InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
                InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD -> FieldKind.PASSWORD
                InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
                InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS -> FieldKind.EMAIL
                InputType.TYPE_TEXT_VARIATION_URI -> FieldKind.URL
                else -> if (inputType and InputType.TYPE_TEXT_FLAG_MULTI_LINE != 0) FieldKind.TEXT else FieldKind.SHORT_TEXT
            }
            else -> FieldKind.TEXT
        }
    }

    enum class Enter { NEWLINE, GO, SEARCH, SEND, NEXT, DONE, PREVIOUS }

    /** What the Enter key does in this field. */
    fun enterOf(inputType: Int, imeOptions: Int): Enter {
        val multiLine = inputType and InputType.TYPE_TEXT_FLAG_MULTI_LINE != 0
        if (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION != 0 || multiLine) return Enter.NEWLINE
        return when (imeOptions and EditorInfo.IME_MASK_ACTION) {
            EditorInfo.IME_ACTION_GO -> Enter.GO
            EditorInfo.IME_ACTION_SEARCH -> Enter.SEARCH
            EditorInfo.IME_ACTION_SEND -> Enter.SEND
            EditorInfo.IME_ACTION_NEXT -> Enter.NEXT
            EditorInfo.IME_ACTION_DONE -> Enter.DONE
            EditorInfo.IME_ACTION_PREVIOUS -> Enter.PREVIOUS
            else -> Enter.NEWLINE
        }
    }

    fun actionOf(enter: Enter): Int = when (enter) {
        Enter.GO -> EditorInfo.IME_ACTION_GO
        Enter.SEARCH -> EditorInfo.IME_ACTION_SEARCH
        Enter.SEND -> EditorInfo.IME_ACTION_SEND
        Enter.NEXT -> EditorInfo.IME_ACTION_NEXT
        Enter.DONE -> EditorInfo.IME_ACTION_DONE
        Enter.PREVIOUS -> EditorInfo.IME_ACTION_PREVIOUS
        Enter.NEWLINE -> EditorInfo.IME_ACTION_NONE
    }
}
