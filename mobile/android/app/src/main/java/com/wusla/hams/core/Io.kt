package com.wusla.hams.core

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Run an engine call off the main thread. Failures come back as a message the user can read. */
suspend fun <T> io(block: HamsEngine.() -> T): Result<T> = withContext(Dispatchers.IO) {
    try {
        Result.success(Hams.engine.block())
    } catch (e: HamsException.Failed) {
        Result.failure(Exception(e.reason))
    } catch (e: Throwable) {
        Result.failure(e)
    }
}
