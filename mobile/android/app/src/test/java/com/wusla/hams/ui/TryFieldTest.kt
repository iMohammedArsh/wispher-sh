package com.wusla.hams.ui

import com.wusla.hams.core.Hams
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

/** The in-app field the orb dictates into: it must behave like a real text field to the engine. */
class TryFieldTest {
    @After
    fun reset() {
        TryField.detach()
        TryField.clear()
    }

    @Test
    fun attachMakesItTheEnginesFieldWithAFreshToken() {
        TryField.attach("com.wusla.hams")
        assertSame(TryField, Hams.field)
        val info = TryField.info()
        assertTrue(info.token != 0uL)
        assertEquals("com.wusla.hams", info.`package`)
    }

    @Test
    fun commitsOnlyForItsOwnToken() {
        TryField.attach("com.wusla.hams")
        val token = TryField.info().token.toLong()
        assertTrue(TryField.commit(token, "Send it tomorrow."))
        assertFalse("another field's token", TryField.commit(token + 1, " nope"))
        assertEquals("Send it tomorrow.", TryField.text.value)
        assertEquals("ow.", TryField.textBeforeCursor(3))
    }

    @Test
    fun undoDeletesFromTheEnd() {
        TryField.attach("com.wusla.hams")
        val token = TryField.info().token.toLong()
        TryField.commit(token, "Hello.")
        TryField.commit(token, " Again.")
        assertTrue(TryField.deleteBeforeCursor(token, 7))
        assertEquals("Hello.", TryField.text.value)
        assertFalse("more than is there", TryField.deleteBeforeCursor(token, 99))
    }

    @Test
    fun afterDetachNothingLandsAndTheEngineLetsGo() {
        TryField.attach("com.wusla.hams")
        val token = TryField.info().token.toLong()
        TryField.detach()
        assertNull(Hams.field)
        assertFalse(TryField.commit(token, "late text"))
        assertEquals("", TryField.text.value)
    }
}
