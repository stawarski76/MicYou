package com.lanrhyme.micyou.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.lanrhyme.micyou.network.PairingScanOutcome
import com.lanrhyme.micyou.network.PairingScanSession
import com.lanrhyme.micyou.network.PairingStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch

class PairingScanViewModel(application: Application) : AndroidViewModel(application) {
    private val session = PairingScanSession(viewModelScope) { value ->
        withContext(Dispatchers.IO) { PairingStore.save(application, value) }
    }
    val outcome = session.outcome
    private val messages = Channel<PairingScanOutcome>(Channel.BUFFERED)
    val notifications = messages.receiveAsFlow()
    init {
        viewModelScope.launch {
            outcome.collect { value ->
                if (value != PairingScanOutcome.IDLE && value != PairingScanOutcome.SCANNING && value != PairingScanOutcome.SAVING) {
                    messages.send(value)
                }
            }
        }
    }
    fun begin() = session.begin()
    fun report(value: PairingScanOutcome) = session.report(value)
    fun accept(contents: String?, format: String?, available: Boolean) = session.accept(contents, format, available)
}
