package com.wusla.hams.ui

/** Where the microphone prompt stands during setup. */
enum class MicAsk {
    /** Not refused yet. */
    Fresh,

    /** Refused once; Android will show the prompt again. */
    Retry,

    /** Refused for good ("Don't ask again", or twice): only the app's settings can turn it on. */
    Blocked,
}

/**
 * The state after the user refused the prompt. `showRationale` is
 * `shouldShowRequestPermissionRationale` read right after the refusal: true
 * means Android will still ask (the first refusal), false means it won't. It
 * must not be read before the first prompt, when it is also false.
 */
fun micAskAfterRefusal(showRationale: Boolean): MicAsk = if (showRationale) MicAsk.Retry else MicAsk.Blocked

enum class DownloadCheck {
    Ok,

    /** Metered network (mobile data or a hotspot): ask first. */
    MobileData,

    /** Not enough free space: refuse, and say how much is needed. */
    NoSpace,
}

/** Free space wanted on top of the file itself (the `.part` is renamed when done, not copied). */
private const val SPACE_MARGIN_MB = 100

/** Is it fine to start a `sizeMb` download now? Space comes first: no choice helps without it. */
fun downloadCheck(sizeMb: Int, freeBytes: Long, onMeteredNetwork: Boolean): DownloadCheck = when {
    freeBytes < (sizeMb + SPACE_MARGIN_MB) * 1_048_576L -> DownloadCheck.NoSpace
    onMeteredNetwork -> DownloadCheck.MobileData
    else -> DownloadCheck.Ok
}
