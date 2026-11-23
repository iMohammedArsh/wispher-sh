package com.wusla.hams

import android.app.Application
import com.wusla.hams.core.Hams

class HamsApp : Application() {
    override fun onCreate() {
        super.onCreate()
        Hams.initAsync(this)
    }

    @Suppress("DEPRECATION") // the trim levels still arrive on every API level we support
    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        // The Whisper model is the big allocation; it reloads (and re-warms) on the
        // next dictation, which costs seconds. Only give it up when the system really
        // needs the memory: not when the app is merely hidden (UI_HIDDEN) or the
        // process moves down the background list, which happens every time the user
        // switches away from the app they are typing in.
        if (level == TRIM_MEMORY_RUNNING_CRITICAL || level >= TRIM_MEMORY_MODERATE) Hams.releaseModel()
    }
}
