package com.wusla.hams.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class SetupLogicTest {
    private val gb = 1_073_741_824L

    @Test
    fun theFirstRefusalCanBeAskedAgainButALaterOneCannot() {
        assertEquals(MicAsk.Retry, micAskAfterRefusal(showRationale = true))
        assertEquals(MicAsk.Blocked, micAskAfterRefusal(showRationale = false))
    }

    @Test
    fun aDownloadOnWifiWithRoomJustStarts() {
        assertEquals(DownloadCheck.Ok, downloadCheck(sizeMb = 466, freeBytes = 5 * gb, onMeteredNetwork = false))
    }

    @Test
    fun aDownloadOnMobileDataAsksFirst() {
        assertEquals(DownloadCheck.MobileData, downloadCheck(466, 5 * gb, onMeteredNetwork = true))
    }

    @Test
    fun noRoomIsRefusedWhateverTheNetwork() {
        assertEquals(DownloadCheck.NoSpace, downloadCheck(466, 300L * 1_048_576, onMeteredNetwork = false))
        assertEquals(DownloadCheck.NoSpace, downloadCheck(466, 300L * 1_048_576, onMeteredNetwork = true))
    }

    @Test
    fun theFileSizePlusTheMarginIsEnough() {
        val needed = (466 + 100) * 1_048_576L
        assertEquals(DownloadCheck.Ok, downloadCheck(466, needed, onMeteredNetwork = false))
        assertEquals(DownloadCheck.NoSpace, downloadCheck(466, needed - 1, onMeteredNetwork = false))
    }

    @Test
    fun setupIsOnlyCompleteOnceHamsIsTheKeyboardInUse() {
        val almost = Setup(keyboardEnabled = true, keyboardSelected = false, micAllowed = true, modelReady = true)
        assertEquals(false, almost.complete)
        assertEquals(1, almost.missing)
        val done = almost.copy(keyboardSelected = true)
        assertEquals(true, done.complete)
        assertEquals(0, done.missing)
        assertEquals(4, Setup().missing)
    }
}
