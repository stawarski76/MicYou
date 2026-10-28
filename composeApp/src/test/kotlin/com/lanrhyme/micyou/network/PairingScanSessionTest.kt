package com.lanrhyme.micyou.network

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class PairingScanSessionTest {
    private val code = "ymic1:${"11".repeat(32)}:${"42".repeat(32)}"

    @Test fun validScanPersistsBeforeReportingSuccessWithoutAUiCollector() = runBlocking {
        val allowSave = CompletableDeferred<Unit>()
        var stored: String? = null
        val session = PairingScanSession(this) { value -> allowSave.await(); stored = value }
        session.begin()
        session.accept(code, "QR_CODE", true)
        yield()
        assertEquals(PairingScanOutcome.SAVING, session.outcome.value)
        assertNull(stored)
        allowSave.complete(Unit)
        coroutineContext[Job]!!.children.toList().joinAll()
        assertEquals(code, stored)
        assertEquals(PairingScanOutcome.SAVED, session.outcome.value)
    }

    @Test fun cancellationInvalidCodesAndBusyAudioHaveExplicitResultsWithoutSaving() = runBlocking {
        var writes = 0
        val session = PairingScanSession(this) { writes++ }
        session.accept(null, null, true)
        assertEquals(PairingScanOutcome.CANCELLED, session.outcome.value)
        session.accept("https://example.com", "QR_CODE", true)
        assertEquals(PairingScanOutcome.INVALID, session.outcome.value)
        session.accept(code, "CODE_128", true)
        assertEquals(PairingScanOutcome.INVALID, session.outcome.value)
        session.accept(code, "QR_CODE", false)
        assertEquals(PairingScanOutcome.UNAVAILABLE, session.outcome.value)
        assertEquals(0, writes)
    }

    @Test fun persistenceFailureNeverReportsPairingSuccess() = runBlocking {
        val session = PairingScanSession(this) { throw IllegalStateException("test failure") }
        session.accept(code, "QR_CODE", true)
        coroutineContext[Job]!!.children.toList().joinAll()
        assertEquals(PairingScanOutcome.FAILED, session.outcome.value)
    }
}
