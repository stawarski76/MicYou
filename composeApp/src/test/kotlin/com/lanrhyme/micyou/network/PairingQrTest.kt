package com.lanrhyme.micyou.network

import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.RGBLuminanceSource
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeReader
import org.junit.Assert.*
import org.junit.Test

class PairingQrTest {
    @Test fun androidDecodesTheDesktopQrWithoutChangingPinOrToken() {
        // Produced by the desktop qrcode library, using a deliberately fake
        // credential. No production identity is embedded in the fixture.
        val lines = requireNotNull(javaClass.getResourceAsStream("/pairing-desktop-qr.txt"))
            .bufferedReader().use { it.readLines() }
        val matrix = lines.drop(1)
        val size = matrix.size * 4
        val pixels = IntArray(size * size) { i ->
            if (matrix[(i / size) / 4][(i % size) / 4] == '1') 0xff000000.toInt() else 0xffffffff.toInt()
        }
        val bitmap = BinaryBitmap(HybridBinarizer(RGBLuminanceSource(size, size, pixels)))
        val decoded = QRCodeReader().decode(bitmap, mapOf(DecodeHintType.PURE_BARCODE to true))
        assertEquals(lines.first(), decoded.text)
        assertEquals("QR_CODE", decoded.barcodeFormat.toString())
        val credentials = PairingCredentials.parse(decoded.text)
        assertArrayEquals(ByteArray(32) { 0x42.toByte() }, credentials.tokenBytes())
    }

    @Test fun arbitraryQrContentsCannotBecomePairingCredentials() {
        val code = "ymic1:${"11".repeat(32)}:${"42".repeat(32)}"
        for (value in listOf("https://example.com/$code", "prefix$code", "$code:extra", "ymic2:${"11".repeat(32)}:${"42".repeat(32)}")) {
            assertTrue(runCatching { PairingCredentials.parse(value) }.isFailure)
        }
    }
}
