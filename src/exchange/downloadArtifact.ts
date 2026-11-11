/** Save a byte artifact through a transient object URL. Browser-only. */
export function downloadBytes(data: Uint8Array, mimeType: string, fileName: string): void {
  const buffer = new ArrayBuffer(data.byteLength)
  new Uint8Array(buffer).set(data)
  const url = URL.createObjectURL(new Blob([buffer], { type: mimeType }))
  const link = document.createElement('a')
  link.href = url
  link.download = fileName
  link.click()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}
