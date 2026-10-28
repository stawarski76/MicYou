package com.lanrhyme.micyou.ui

import androidx.compose.foundation.layout.*
import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
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
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
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
    var invalidQr by remember { mutableStateOf(false) }
    var cameraDenied by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { saved = withContext(Dispatchers.IO) { runCatching { PairingStore.load(context) }.isSuccess } }
    val valid = remember(code) { runCatching { PairingCredentials.parse(code) }.isSuccess }
    fun savePairing(value: String) {
        busy = true; failed = false; invalidQr = false; cameraDenied = false
        scope.launch {
            try { withContext(Dispatchers.IO) { PairingStore.save(context, value) }; saved = true; code = "" }
            catch (_: Exception) { failed = true }
            finally { busy = false }
        }
    }
    val scanner = rememberLauncherForActivityResult(ScanContract()) { result ->
        busy = false
        val contents = result.contents
        // Cancellation and invalid QR codes never remove an existing pairing.
        // A scan only imports credentials; it never starts the microphone.
        if (enabled && contents != null) {
            if (result.formatName != ScanOptions.QR_CODE || runCatching { PairingCredentials.parse(contents) }.isFailure) {
                invalidQr = true
            } else savePairing(contents)
        }
    }
    fun launchScanner() {
        try {
            scanner.launch(ScanOptions().apply {
                setCaptureActivity(PairingCaptureActivity::class.java)
                setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                setPrompt(context.getString(R.string.securePairingScanPrompt))
                setBeepEnabled(false); setBarcodeImageEnabled(false)
                setOrientationLocked(false); setTimeout(60000)
            })
        } catch (_: Exception) { busy = false; failed = true }
    }
    val cameraPermission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted && enabled) launchScanner()
        else { busy = false; cameraDenied = !granted }
    }
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(stringResource(R.string.securePairingTitle), style = MaterialTheme.typography.titleMedium)
            Text(stringResource(R.string.securePairingDescription))
            if (saved) Text(stringResource(R.string.securePairingSaved))
            Button(enabled = enabled && !busy && Build.VERSION.SDK_INT >= 23, onClick = {
                busy = true; failed = false; invalidQr = false; cameraDenied = false
                if (ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
                    launchScanner()
                } else {
                    try { cameraPermission.launch(Manifest.permission.CAMERA) }
                    catch (_: Exception) { busy = false; failed = true }
                }
            }) { Text(stringResource(R.string.securePairingScan)) }
            if (invalidQr) Text(stringResource(R.string.securePairingInvalidQr), color = MaterialTheme.colorScheme.error)
            if (cameraDenied) Text(stringResource(R.string.securePairingCameraDenied), color = MaterialTheme.colorScheme.error)
            OutlinedTextField(value = code, onValueChange = { if (it.length <= 256) code = it },
                modifier = Modifier.fillMaxWidth(), enabled = enabled && !busy,
                label = { Text(stringResource(R.string.securePairingCode)) },
                visualTransformation = PasswordVisualTransformation(), singleLine = true)
            if (failed) Text(stringResource(R.string.securePairingFailed), color = MaterialTheme.colorScheme.error)
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Button(enabled = enabled && !busy && valid, onClick = {
                    savePairing(code)
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
