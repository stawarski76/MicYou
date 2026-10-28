package com.lanrhyme.micyou.network

import java.security.MessageDigest
import java.security.cert.CertificateException
import java.security.cert.X509Certificate
import javax.net.ssl.X509TrustManager

/** A credential, never a data class: generated toString/copy must not expose it. */
class PairingCredentials private constructor(private val fingerprint: ByteArray, private val token: ByteArray) {
    fun tokenBytes(): ByteArray = token.copyOf()

    fun trustManager(): X509TrustManager = object : X509TrustManager {
        override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
        override fun checkClientTrusted(chain: Array<X509Certificate>?, authType: String?) {
            throw CertificateException("Client certificate verification is not supported")
        }
        override fun checkServerTrusted(chain: Array<X509Certificate>?, authType: String?) {
            val leaf = chain?.firstOrNull() ?: throw CertificateException("Missing server certificate")
            leaf.checkValidity()
            val actual = MessageDigest.getInstance("SHA-256").digest(leaf.encoded)
            if (!MessageDigest.isEqual(fingerprint, actual)) {
                throw CertificateException("The server does not match the paired computer")
            }
        }
    }

    companion object {
        val AUTH_MAGIC = byteArrayOf(89, 77, 73, 67, 118, 49, 0, 0)
        val AUTH_OK = byteArrayOf(89, 77, 79, 75)
        private val FORMAT = Regex("ymic1:[0-9a-fA-F]{64}:[0-9a-fA-F]{64}")
        fun parse(value: String): PairingCredentials {
            val code = value.trim()
            require(FORMAT.matches(code)) { "Invalid pairing code" }
            val fields = code.split(':')
            fun decode(hex: String) = ByteArray(32) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
            return PairingCredentials(decode(fields[1]), decode(fields[2]))
        }
    }
}
