import {measuredPhotoInput, type PhotoCalibrationGroup} from './calibration'
import type {PhotoPixels} from './kernel'

/** Optional JPEG EXIF hint, not calibrated intrinsics. Unknown cameras stay explicit. */
export function photoFocalHint(buffer: ArrayBuffer): number | null {
  const view = new DataView(buffer)
  if (view.byteLength < 4 || view.getUint16(0) !== 0xffd8) return null
  try {
    let offset = 2
    while (offset + 4 < view.byteLength) {
      if (view.getUint8(offset) !== 0xff) break
      const marker = view.getUint8(offset + 1)
      if (marker === 0xda || marker === 0xd9) break
      const length = view.getUint16(offset + 2)
      if (length < 2 || offset + length + 2 > view.byteLength) break
      const isExif = marker === 0xe1 && view.getUint32(offset + 4) === 0x45786966 && view.getUint16(offset + 8) === 0
      if (isExif) return tiffFocalHint(buffer, offset + 10, offset + length + 2)
      offset += length + 2
    }
  } catch { /* Truncated or malformed metadata is an absent hint. */ }
  return null
}

function tiffFocalHint(buffer: ArrayBuffer, base: number, end: number): number | null {
  // Restrict all relative offsets to this APP1 segment, not the rest of the JPEG.
  const view = new DataView(buffer, base, end - base)
  const order = view.getUint16(0)
  if (order !== 0x4949 && order !== 0x4d4d) return null
  const little = order === 0x4949
  const u16 = (offset: number) => view.getUint16(offset, little)
  const u32 = (offset: number) => view.getUint32(offset, little)
  if (u16(2) !== 42) return null
  const entries = (offset: number): number[] => {
    const count = u16(offset)
    if (count > 512 || offset + 2 + count * 12 > view.byteLength) throw new Error('Invalid EXIF entries')
    return Array.from({length: count}, (_, i) => offset + 2 + i * 12)
  }
  let model = '', focal: number | null = null, exif = 0
  for (const entry of entries(u32(4))) {
    const tag = u16(entry)
    if (tag === 0x8769) exif = u32(entry + 8)
    if (tag === 0x0110 && u16(entry + 2) === 2) {
      const count = u32(entry + 4)
      const length = Math.min(count, 128)
      const offset = count <= 4 ? entry + 8 : u32(entry + 8)
      if (offset + length > view.byteLength) return null
      model = new TextDecoder().decode(new Uint8Array(buffer, base + offset, length)).replace(/\0/g, '')
    }
  }
  if (exif) for (const entry of entries(exif)) {
    if (u16(entry) === 0xa405 && u16(entry + 2) === 3) {
      const equivalent = u16(entry + 8)
      if (equivalent > 0 && equivalent < 2000) return equivalent
    }
    if (u16(entry) === 0x920a && u16(entry + 2) === 5) {
      const offset = u32(entry + 8)
      focal = u32(offset) / u32(offset + 4)
    }
  }
  const knownFullFrame = /Canon EOS (R8|R5|R6|R |5D|6D|1D X)/i.test(model)
  return focal && Number.isFinite(focal) && knownFullFrame ? focal : null
}

export const MAX_PHOTOS = 24
export const MAX_PHOTO_FILE_BYTES = 40 * 1024 * 1024

export interface ImportedPhoto {
  readonly file: File
  readonly url: string
  readonly equivalent: number
  readonly hint: boolean
  readonly calibrationGroupId?: string
}

interface ObjectUrls {
  createObjectURL(blob: Blob): string
  revokeObjectURL(url: string): void
}

export class PhotoInputError extends Error {
  constructor(readonly code: 'limit' | 'size' | 'focal', readonly filename = '') {
    super(code === 'limit' ? `At most ${MAX_PHOTOS} photos are supported`
      : code === 'size' ? `${filename}: file exceeds 40 MiB`
      : 'Focal equivalent must be between 8 and 1000 mm')
  }
}

export function validPhotoFocal(value: number): boolean {
  return Number.isFinite(value) && value >= 8 && value <= 1000
}

/** Owns thumbnail URLs. A failed or disposed import cannot partially publish a batch. */
export class PhotoCollection {
  private photos: ImportedPhoto[] = []
  private disposed = false
  private pending = false

  constructor(private readonly urls: ObjectUrls = URL) {}

  get entries(): readonly ImportedPhoto[] { return this.photos }

  async add(files: readonly File[]): Promise<void> {
    if (this.disposed) throw new DOMException('Photo collection is disposed', 'AbortError')
    if (this.pending) throw new Error('Photo import is already running')
    if (this.photos.length + files.length > MAX_PHOTOS) throw new PhotoInputError('limit')
    for (const file of files) if (file.size > MAX_PHOTO_FILE_BYTES) throw new PhotoInputError('size', file.name)

    this.pending = true
    const incoming: ImportedPhoto[] = []
    try {
      // Start every read up front; results still publish in input order.
      const reads = files.map(file => {
        const reading = file.arrayBuffer()
        reading.catch(() => { /* A rejected batch abandons the remaining reads. */ })
        return reading
      })
      for (let index = 0; index < files.length; index++) {
        const file = files[index]!
        const data = await reads[index]!
        if (this.disposed) throw new DOMException('Photo import was cancelled', 'AbortError')
        const focal = photoFocalHint(data)
        const hint = focal !== null && validPhotoFocal(focal)
        incoming.push({file, url: this.urls.createObjectURL(file), equivalent: hint ? focal : 50, hint})
      }
      this.photos = [...this.photos, ...incoming]
    } catch (error) {
      for (const photo of incoming) this.urls.revokeObjectURL(photo.url)
      throw error
    } finally {
      this.pending = false
    }
  }

  remove(index: number): void {
    const photo = this.photos[index]
    if (!photo) return
    this.urls.revokeObjectURL(photo.url)
    this.photos = this.photos.filter((_, i) => i !== index)
  }

  setFocal(index: number, equivalent: number): void {
    if (!validPhotoFocal(equivalent)) throw new PhotoInputError('focal')
    this.photos = this.photos.map((photo, i) => i === index ? {...photo, equivalent, hint: false} : photo)
  }

  setCalibration(index: number, calibrationGroupId?: string): void {
    this.photos = this.photos.map((photo, i) => i === index ? {...photo, calibrationGroupId} : photo)
  }

  dispose(): void {
    if (this.disposed) return
    this.disposed = true
    for (const photo of this.photos) this.urls.revokeObjectURL(photo.url)
    this.photos = []
  }
}

/** Decodes to the kernel's RGB/pinhole contract; cancellation also closes an in-flight bitmap. */
export async function decodePhoto(
  file: File,
  equivalent: number,
  maxSide = 960,
  signal?: AbortSignal,
  calibration?: PhotoCalibrationGroup,
): Promise<PhotoPixels> {
  if (!calibration && !validPhotoFocal(equivalent)) throw new PhotoInputError('focal')
  if (!Number.isInteger(maxSide) || maxSide < 48 || maxSide > 2048) throw new Error('Invalid photo resolution')
  signal?.throwIfAborted()
  const bitmap = await createImageBitmap(file, {imageOrientation: 'from-image'})
  try {
    signal?.throwIfAborted()
    if (bitmap.width * bitmap.height > 50_000_000) throw new Error('Photo exceeds 50 megapixels')
    const measured = calibration ? measuredPhotoInput(calibration, bitmap.width, bitmap.height) : undefined
    const scale = Math.min(1, maxSide / Math.max(bitmap.width, bitmap.height))
    const width = Math.round(bitmap.width * scale)
    const height = Math.round(bitmap.height * scale)
    if (width < 48 || height < 48) throw new Error('Photo is too small or narrow after resizing (minimum 48 pixels per side)')
    const canvas = document.createElement('canvas')
    canvas.width = width
    canvas.height = height
    const context = canvas.getContext('2d', {willReadFrequently: true})
    if (!context) throw new Error('Image decoding is unavailable')
    context.drawImage(bitmap, 0, 0, width, height)
    const rgba = context.getImageData(0, 0, width, height).data
    const rgb = new Uint8Array(width * height * 3)
    // One 32-bit read per pixel (little-endian RGBA); alpha is dropped.
    const pixels = new Uint32Array(rgba.buffer, rgba.byteOffset, width * height)
    for (let source = 0, target = 0; source < pixels.length; source++, target += 3) {
      const pixel = pixels[source]!
      rgb[target] = pixel & 255
      rgb[target + 1] = (pixel >>> 8) & 255
      rgb[target + 2] = pixel >>> 16
    }
    const focal = calibration
      ? Math.sqrt(calibration.fx * width / bitmap.width * calibration.fy * height / bitmap.height)
      : equivalent * Math.hypot(width, height) / Math.hypot(36, 24)
    return {width, height, rgb, focal, ...(measured ? {calibration: measured} : {})}
  } finally {
    bitmap.close()
  }
}
