package com.lanrhyme.micyou.ui

import android.os.Build
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.lanrhyme.micyou.R
import com.lanrhyme.micyou.network.PairingCredentials
import com.lanrhyme.micyou.network.PairingScanOutcome
import com.lanrhyme.micyou.network.PairingStore
import com.lanrhyme.micyou.viewmodel.PairingScanViewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun PairingSetupCard(enabled: Boolean) {
    val context = LocalContext.current.applicationContext
    val scope = rememberCoroutineScope()
    val model: PairingScanViewModel = viewModel()
    val outcome by model.outcome.collectAsState()
    val scan = LocalPairingScanner.current
    var code by remember { mutableStateOf("") }
    var saved by remember { mutableStateOf(false) }
    var forgetting by remember { mutableStateOf(false) }
    var failed by remember { mutableStateOf(false) }
    var storageRevision by remember { mutableIntStateOf(0) }
    val busy = forgetting || outcome == PairingScanOutcome.SCANNING || outcome == PairingScanOutcome.SAVING
    LaunchedEffect(Unit) {
        val revision = storageRevision
        val exists = withContext(Dispatchers.IO) { runCatching { PairingStore.load(context) }.isSuccess }
        if (revision == storageRevision && model.outcome.value != PairingScanOutcome.SAVED) saved = exists
    }
    LaunchedEffect(outcome) {
        if (outcome == PairingScanOutcome.SAVED) { saved = true; code = ""; failed = false }
    }
    val valid = remember(code) { runCatching { PairingCredentials.parse(code) }.isSuccess }
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(stringResource(R.string.securePairingTitle), style = MaterialTheme.typography.titleMedium)
            Text(stringResource(R.string.securePairingDescription))
            if (saved && outcome != PairingScanOutcome.SAVED) Text(stringResource(R.string.securePairingSaved))
            outcome.messageResource()?.let { message ->
                Text(stringResource(message), color = when (outcome) {
                    PairingScanOutcome.SAVED -> MaterialTheme.colorScheme.primary
                    PairingScanOutcome.SAVING, PairingScanOutcome.CANCELLED -> MaterialTheme.colorScheme.onSurfaceVariant
                    else -> MaterialTheme.colorScheme.error
                })
            }
            Button(enabled = enabled && !busy && scan != null && Build.VERSION.SDK_INT >= 23, onClick = {
                failed = false
                scan?.invoke()
            }) { Text(stringResource(R.string.securePairingScan)) }
            OutlinedTextField(value = code, onValueChange = { if (it.length <= 256) code = it },
                modifier = Modifier.fillMaxWidth(), enabled = enabled && !busy,
                label = { Text(stringResource(R.string.securePairingCode)) },
                visualTransformation = PasswordVisualTransformation(), singleLine = true)
            if (failed) Text(stringResource(R.string.securePairingFailed), color = MaterialTheme.colorScheme.error)
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Button(enabled = enabled && !busy && valid, onClick = {
                    failed = false
                    model.accept(code, "QR_CODE", enabled)
                }) { Text(stringResource(R.string.securePairingSave)) }
                // Forgetting must recover invalidated Keystore entries too.
                TextButton(enabled = enabled && !busy, onClick = {
                    storageRevision++
                    forgetting = true; failed = false
                    scope.launch {
                        try {
                            withContext(Dispatchers.IO) { PairingStore.forget(context) }
                            saved = false; code = ""; model.report(PairingScanOutcome.IDLE)
                        } catch (_: Exception) { failed = true }
                        finally { forgetting = false }
                    }
                }) { Text(stringResource(R.string.securePairingForget)) }
            }
        }
    }
}
