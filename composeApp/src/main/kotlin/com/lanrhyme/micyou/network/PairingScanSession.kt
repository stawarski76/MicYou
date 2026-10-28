package com.lanrhyme.micyou.network

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

enum class PairingScanOutcome { IDLE, SCANNING, SAVING, SAVED, INVALID, CANCELLED, CAMERA_DENIED, UNAVAILABLE, FAILED }

/** Owned by the Activity ViewModel, not by a disposable settings-list item. */
class PairingScanSession(private val scope: CoroutineScope, private val persist: suspend (String) -> Unit) {
    private val current = MutableStateFlow(PairingScanOutcome.IDLE)
    val outcome = current.asStateFlow()

    fun begin(): Boolean {
        if (current.value == PairingScanOutcome.SAVING) return false
        current.value = PairingScanOutcome.SCANNING
        return true
    }
    fun report(value: PairingScanOutcome) {
        if (current.value != PairingScanOutcome.SAVING) current.value = value
    }
    fun accept(contents: String?, format: String?, available: Boolean) {
        if (current.value == PairingScanOutcome.SAVING) return
        if (contents == null) { report(PairingScanOutcome.CANCELLED); return }
        if (!available) { report(PairingScanOutcome.UNAVAILABLE); return }
        if (format != "QR_CODE" || runCatching { PairingCredentials.parse(contents) }.isFailure) {
            report(PairingScanOutcome.INVALID); return
        }
        current.value = PairingScanOutcome.SAVING
        scope.launch {
            try { persist(contents); current.value = PairingScanOutcome.SAVED }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { current.value = PairingScanOutcome.FAILED }
        }
    }
}
