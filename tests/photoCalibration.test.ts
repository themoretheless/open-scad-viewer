import {afterEach, describe, expect, it, vi} from 'vitest'
import {measuredPhotoInput, parsePhotoCalibration, photoCalibrationExampleJson} from '../src/services/photogrammetry/calibration'
import {decodePhoto, PhotoCollection} from '../src/services/photogrammetry/input'
import {photoReportJson} from '../src/services/photogrammetry/report'

const example = () => parsePhotoCalibration(photoCalibrationExampleJson())
afterEach(() => vi.unstubAllGlobals())

describe('measured calibration import', () => {
  it('retains both explicit groups and their measurement provenance', () => {
    const document = example()
    expect(document.groups.map(group => group.id)).toEqual(['synthetic-a', 'synthetic-b'])
    expect(document.groups[0].distortion).toMatchObject({model: 'brown-conrady', p1: 0.015, p2: -0.01})
    expect(document.groups[0].source).toContain('not Canon/iPhone')
  })

  it('rejects ambiguous models, duplicate identities, missing values and non-finite intrinsics', () => {
    const changes = [
      (d: any) => { d.groups[0].distortion.model = 'opencv-fisheye' },
      (d: any) => { d.groups[1].id = d.groups[0].id },
      (d: any) => { d.groups[0].fx = Infinity },
      (d: any) => { delete d.groups[0].distortion.p2 },
      (d: any) => { d.coordinateSystem = 'raw-sensor' },
      (d: any) => { d.groups[0].imageWidth = 3.5 },
      (d: any) => { d.groups[0].rational = true },
    ]
    for (const change of changes) {
      const document = example()
      change(document)
      expect(() => parsePhotoCalibration(JSON.stringify(document))).toThrow()
    }
    expect(() => parsePhotoCalibration(' '.repeat(128 * 1024 + 1))).toThrow('128 KiB')
  })

  it('assigns calibration explicitly and preserves it when another photo is removed', async () => {
    const photos = new PhotoCollection({createObjectURL: () => 'blob:test', revokeObjectURL: vi.fn()})
    await photos.add([new File(['a'], 'one.jpg'), new File(['b'], 'two.jpg')])
    expect(photos.entries.every(photo => photo.calibrationGroupId === undefined)).toBe(true)
    photos.setCalibration(0, 'synthetic-a')
    photos.setCalibration(1, 'synthetic-b')
    photos.remove(0)
    expect(photos.entries[0].file.name).toBe('two.jpg')
    expect(photos.entries[0].calibrationGroupId).toBe('synthetic-b')
    photos.setCalibration(0)
    expect(photos.entries[0].calibrationGroupId).toBeUndefined()
    photos.dispose()
  })
})

describe('calibration coordinates and photo decoding', () => {
  it('keeps original intrinsics unchanged for the single Rust resize/rectify path', async () => {
    const group = example().groups[0]
    const close = vi.fn()
    vi.stubGlobal('createImageBitmap', async () => ({width: 320, height: 240, close}))
    const canvas = {width: 0, height: 0, getContext: () => ({drawImage: vi.fn(),
      getImageData: () => ({data: new Uint8ClampedArray(160 * 120 * 4).fill(128)})})}
    vi.stubGlobal('document', {createElement: () => canvas})
    const decoded = await decodePhoto(new File([''], 'test.png'), NaN, 160, undefined, group)
    expect(decoded.width).toBe(160)
    expect(decoded.height).toBe(120)
    expect(decoded.calibration).toEqual({group, sourceWidth: 320, sourceHeight: 240})
    expect(decoded.calibration!.group.fx).toBe(245)
    expect(decoded.focal).toBeCloseTo(Math.sqrt(245 * 270) / 2)
    expect(decoded.rgb).toHaveLength(160 * 120 * 3)
    expect(close).toHaveBeenCalledOnce()
  })

  it('rejects mismatched orientation or crop before reading pixels and closes the bitmap', async () => {
    const close = vi.fn(), createElement = vi.fn()
    vi.stubGlobal('createImageBitmap', async () => ({width: 240, height: 320, close}))
    vi.stubGlobal('document', {createElement})
    await expect(decodePhoto(new File([''], 'portrait.png'), 50, 960, undefined, example().groups[0])).rejects.toThrow('oriented photo')
    expect(close).toHaveBeenCalledOnce()
    expect(createElement).not.toHaveBeenCalled()
  })

  it('cancels the measured path before an in-flight bitmap can be published', async () => {
    const abort = new AbortController(), close = vi.fn()
    let complete!: (value: unknown) => void
    vi.stubGlobal('createImageBitmap', () => new Promise(resolve => { complete = resolve }))
    const decoding = decodePhoto(new File([''], 'test.png'), 50, 960, abort.signal, example().groups[0])
    abort.abort()
    complete({width: 320, height: 240, close})
    await expect(decoding).rejects.toMatchObject({name: 'AbortError'})
    expect(close).toHaveBeenCalledOnce()
  })

  it('exports imported coefficients and coordinate provenance without pixel buffers', () => {
    const calibration = measuredPhotoInput(example().groups[0], 320, 240)
    const text = photoReportJson({settings: {dense: true, resolution: 128, maxImageSide: 960},
      inputs: [{image: 0, name: 'test.png', bytes: 100, equivalent: 50, width: 320, height: 240, focalPixels: 250, calibration}],
      sparse: null, dense: null, timings: null})
    expect(JSON.parse(text).inputs[0].calibration).toEqual(calibration)
    expect(text).not.toContain('rgb')
  })
})
