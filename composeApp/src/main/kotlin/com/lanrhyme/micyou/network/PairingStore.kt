package com.lanrhyme.micyou.network

import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.io.IOException
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** No plaintext preferences or backups; decryption failure requires explicit re-pairing. */
object PairingStore {
    private const val ALIAS = "yelset_mic_pairing_v1"
    private val AAD = "yelset-mic/pairing/v1".toByteArray(Charsets.UTF_8)
    private fun file(context: Context) = AtomicFile(java.io.File(context.noBackupFilesDir, "yelset-pairing.bin"))
    private fun requireSupported() { check(Build.VERSION.SDK_INT >= 23) { "Secure Wi-Fi requires Android 6 or later; USB is still available" } }
    private fun key(create: Boolean): SecretKey {
        requireSupported()
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        if (store.containsAlias(ALIAS)) return store.getKey(ALIAS, null) as SecretKey
        check(create) { "Pairing key unavailable; pair this phone again" }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setRandomizedEncryptionRequired(true).build())
        return generator.generateKey()
    }

    @Synchronized
    fun load(context: Context): PairingCredentials {
        requireSupported()
        val bytes = file(context).openRead().use { input ->
            val data = ByteArray(1025)
            var used = 0
            while (used < data.size) { val count = input.read(data, used, data.size - used); if (count < 0) break; used += count }
            check(used in 30..1024) { "Invalid protected pairing file" }; data.copyOf(used)
        }
        check(bytes[0] == 1.toByte()) { "Unsupported protected pairing version" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(false), GCMParameterSpec(128, bytes.copyOfRange(1, 13)))
        cipher.updateAAD(AAD)
        val value = cipher.doFinal(bytes.copyOfRange(13, bytes.size)).toString(Charsets.UTF_8)
        return PairingCredentials.parse(value)
    }

    @Synchronized
    fun save(context: Context, value: String) {
        requireSupported()
        PairingCredentials.parse(value)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key(true)); cipher.updateAAD(AAD)
        check(cipher.iv.size == 12) { "Unexpected AES-GCM IV size" }
        val encrypted = byteArrayOf(1) + cipher.iv + cipher.doFinal(value.trim().toByteArray(Charsets.UTF_8))
        val target = file(context)
        val output = target.startWrite()
        try { output.write(encrypted); target.finishWrite(output) }
        catch (failure: Exception) { target.failWrite(output); throw IOException("Could not save protected pairing", failure) }
    }

    @Synchronized
    fun forget(context: Context) {
        file(context).delete()
        if (Build.VERSION.SDK_INT >= 23) KeyStore.getInstance("AndroidKeyStore").apply { load(null); deleteEntry(ALIAS) }
    }
}
