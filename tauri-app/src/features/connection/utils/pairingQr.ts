import QRCode from 'qrcode';

/** Encode locally: the pairing credential must never reach a QR web service. */
export async function pairingQrDataUrl(code: string): Promise<string> {
  if (!/^ymic1:[0-9a-fA-F]{64}:[0-9a-fA-F]{64}$/.test(code)) {
    throw new Error('Invalid pairing credential');
  }
  return QRCode.toDataURL(code, {
    errorCorrectionLevel: 'M', margin: 4, width: 280,
    color: { dark: '#000000ff', light: '#ffffffff' },
  });
}
