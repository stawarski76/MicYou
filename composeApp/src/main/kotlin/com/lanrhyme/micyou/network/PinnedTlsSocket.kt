package com.lanrhyme.micyou.network

import io.ktor.network.sockets.Socket
import io.ktor.network.sockets.SocketAddress
import io.ktor.network.sockets.InetSocketAddress
import io.ktor.utils.io.ByteChannel
import io.ktor.utils.io.WriterJob
import io.ktor.utils.io.ReaderJob
import io.ktor.utils.io.writer
import io.ktor.utils.io.reader
import io.ktor.utils.io.writeFully
import io.ktor.utils.io.jvm.javaio.copyTo
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.withContext
import java.util.concurrent.atomic.AtomicBoolean
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket
import kotlin.coroutines.CoroutineContext

/** OS TLS behind the existing Ktor Socket lifetime/close contract. No plaintext fallback. */
class PinnedTlsSocket(credentials: PairingCredentials, parent: CoroutineContext) : Socket {
    private val closed = AtomicBoolean(false)
    private val reading = AtomicBoolean(false)
    private val writing = AtomicBoolean(false)
    private val delegate = (SSLContext.getInstance("TLS").apply {
        init(null, arrayOf(credentials.trustManager()), null)
    }.socketFactory.createSocket() as SSLSocket).apply {
        enabledProtocols = supportedProtocols.filter { it == "TLSv1.2" || it == "TLSv1.3" }.toTypedArray()
        require(enabledProtocols.isNotEmpty()) { "TLS 1.2 or later is required" }
        soTimeout = 10000; tcpNoDelay = true; keepAlive = true
    }
    override val socketContext = SupervisorJob(parent[Job])
    override val coroutineContext: CoroutineContext = parent.minusKey(Job) + socketContext + Dispatchers.IO
    override val localAddress: SocketAddress get() = InetSocketAddress(delegate.localAddress.hostAddress ?: "0.0.0.0", delegate.localPort)
    override val remoteAddress: SocketAddress get() = InetSocketAddress(delegate.inetAddress.hostAddress ?: error("Socket is not connected"), delegate.port)

    suspend fun connect(host: String, port: Int) = withContext(Dispatchers.IO) {
        try {
            delegate.connect(java.net.InetSocketAddress(host, port), 10000)
            // The pinned TrustManager runs during this handshake, before any
            // application byte (in particular the pairing token) can be sent.
            delegate.startHandshake()
        } catch (failure: Exception) { close(); throw failure }
    }

    override fun attachForReading(channel: ByteChannel): WriterJob {
        check(reading.compareAndSet(false, true)) { "Read channel already attached" }
        return writer(coroutineContext = Dispatchers.IO, channel = channel) {
            try {
                val source = delegate.inputStream; val buffer = ByteArray(8192)
                while (true) { val count = source.read(buffer); if (count < 0) break; channel.writeFully(buffer, 0, count) }
            } finally { close() }
        }
    }
    override fun attachForWriting(channel: ByteChannel): ReaderJob {
        check(writing.compareAndSet(false, true)) { "Write channel already attached" }
        return reader(coroutineContext = Dispatchers.IO, channel = channel) {
            try { channel.copyTo(delegate.outputStream) } finally { close() }
        }
    }
    override fun close() {
        if (closed.compareAndSet(false, true)) {
            try { delegate.close() } finally { socketContext.cancel() }
        }
    }
    override fun dispose() { close() }
}
