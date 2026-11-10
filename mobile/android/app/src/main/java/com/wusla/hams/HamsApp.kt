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
        // The Whisper model is the big allocation; it reloads on the next dictation.
        if (level >= TRIM_MEMORY_RUNNING_LOW) Hams.releaseModel()
    }
}
