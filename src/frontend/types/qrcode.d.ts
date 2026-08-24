declare module 'qrcode' {
  export interface QRCodeToDataURLOptions {
    errorCorrectionLevel?: 'L' | 'M' | 'Q' | 'H'
    width?: number
    margin?: number
    color?: {
      dark?: string
      light?: string
    }
  }

  const QRCode: {
    toDataURL: (text: string, options?: QRCodeToDataURLOptions) => Promise<string>
  }

  export default QRCode
}
