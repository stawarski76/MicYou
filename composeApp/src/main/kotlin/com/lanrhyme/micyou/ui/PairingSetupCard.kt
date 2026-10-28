package com.lanrhyme.micyou.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import com.lanrhyme.micyou.R
import com.lanrhyme.micyou.network.PairingCredentials
import com.lanrhyme.micyou.network.PairingStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun PairingSetupCard(enabled: Boolean) {
    val context = LocalContext.current.applicationContext
    val scope = rememberCoroutineScope()
    var code by remember { mutableStateOf("") }
    var saved by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var failed by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { saved = withContext(Dispatchers.IO) { runCatching { PairingStore.load(context) }.isSuccess } }
    val valid = remember(code) { runCatching { PairingCredentials.parse(code) }.isSuccess }
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(stringResource(R.string.securePairingTitle), style = MaterialTheme.typography.titleMedium)
            Text(stringResource(R.string.securePairingDescription))
            if (saved) Text(stringResource(R.string.securePairingSaved))
            OutlinedTextField(value = code, onValueChange = { if (it.length <= 256) code = it },
                modifier = Modifier.fillMaxWidth(), enabled = enabled && !busy,
                label = { Text(stringResource(R.string.securePairingCode)) },
                visualTransformation = PasswordVisualTransformation(), singleLine = true)
            if (failed) Text(stringResource(R.string.securePairingFailed), color = MaterialTheme.colorScheme.error)
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Button(enabled = enabled && !busy && valid, onClick = {
                    busy = true; failed = false
                    scope.launch {
                        try { withContext(Dispatchers.IO) { PairingStore.save(context, code) }; saved = true; code = "" }
                        catch (_: Exception) { failed = true }
                        finally { busy = false }
                    }
                }) { Text(stringResource(R.string.securePairingSave)) }
                // Explicit forgetting must also recover an unreadable or invalidated
                // Keystore entry; successful decryption is not a prerequisite.
                TextButton(enabled = enabled && !busy, onClick = {
                    busy = true; failed = false
                    scope.launch {
                        try { withContext(Dispatchers.IO) { PairingStore.forget(context) }; saved = false; code = "" }
                        catch (_: Exception) { failed = true }
                        finally { busy = false }
                    }
                }) { Text(stringResource(R.string.securePairingForget)) }
            }
        }
    }
}
