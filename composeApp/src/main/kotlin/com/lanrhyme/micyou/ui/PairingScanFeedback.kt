package com.lanrhyme.micyou.ui

import androidx.compose.runtime.staticCompositionLocalOf
import com.lanrhyme.micyou.R
import com.lanrhyme.micyou.network.PairingScanOutcome

val LocalPairingScanner = staticCompositionLocalOf<(() -> Unit)?> { null }

fun PairingScanOutcome.messageResource(): Int? = when (this) {
    PairingScanOutcome.SAVED -> R.string.securePairingScanSaved
    PairingScanOutcome.INVALID -> R.string.securePairingInvalidQr
    PairingScanOutcome.CANCELLED -> R.string.securePairingScanCancelled
    PairingScanOutcome.CAMERA_DENIED -> R.string.securePairingCameraDenied
    PairingScanOutcome.UNAVAILABLE -> R.string.securePairingScanUnavailable
    PairingScanOutcome.FAILED -> R.string.securePairingFailed
    PairingScanOutcome.SAVING -> R.string.securePairingSaving
    else -> null
}
