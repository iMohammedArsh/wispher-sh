package com.wusla.hams.core

import org.junit.Assert.assertEquals
import org.junit.Test

class WhisperCodesTest {
    private val known = setOf("en", "ml", "he", "id", "no", "tl", "jw", "hi")

    @Test
    fun phoneLanguagesBecomeWhisperCodes() {
        assertEquals(listOf("en", "ml"), whisperCodes(listOf("en-IN", "ml-IN", "en-US"), known))
        assertEquals(listOf("he", "id", "no", "tl", "jw"), whisperCodes(listOf("iw-IL", "in", "nb-NO", "fil-PH", "jv"), known))
    }

    @Test
    fun languagesWhisperDoesNotKnowAreDropped() {
        assertEquals(listOf("hi"), whisperCodes(listOf("xx-YY", "hi-IN"), known))
    }
}
