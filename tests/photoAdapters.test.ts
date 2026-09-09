import {afterEach, describe, expect, it, vi} from 'vitest'
import {decodePhoto, photoFocalHint, PhotoCollection, validPhotoFocal} from '../src/services/photoInput'
import {photoPly, photoScadSource} from '../src/services/photoExport'
import {photoRegistrationReason, photoReportJson} from '../src/services/photoReport'
import {PhotoPreview} from '../src/services/photoPreview'
import type {PhotoSurface} from '../src/services/photogrammetryKernel'

const tetrahedron: PhotoSurface = {
  positions: [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]],
  colors: [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]],
  triangles: [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
}
function objectUrls() {
  let sequence = 0
  return {createObjectURL: vi.fn(() => `blob:photo-${sequence++}`), revokeObjectURL: vi.fn()}
}
function imageFile(name = 'photo.jpg') { return new File([new Uint8Array([1, 2, 3])], name) }

afterEach(() => vi.unstubAllGlobals())

describe('photo input resource ownership', () => {
  it('reads valid EXIF but refuses offsets outside the metadata segment', () => {
    const bytes = new Uint8Array(100)
    const view = new DataView(bytes.buffer)
    bytes.set([255, 216, 255, 225, 0, 52, 69, 120, 105, 102, 0, 0])
    const base = 12
    view.setUint16(base, 0x4949)
    view.setUint16(base + 2, 42, true)
    view.setUint32(base + 4, 8, true)
    view.setUint16(base + 8, 1, true)
    view.setUint16(base + 10, 0x8769, true)
    view.setUint32(base + 18, 26, true)
    view.setUint16(base + 26, 1, true)
    view.setUint16(base + 28, 0xa405, true)
    view.setUint16(base + 30, 3, true)
    view.setUint32(base + 32, 1, true)
    view.setUint16(base + 36, 35, true)
    expect(photoFocalHint(bytes.buffer)).toBe(35)
    bytes.set(bytes.slice(base + 26, base + 40), base + 60)
    view.setUint32(base + 18, 60, true)
    expect(photoFocalHint(bytes.buffer)).toBeNull()
  })

  it('rolls back a failed batch while retaining existing thumbnails', async () => {
    const urls = objectUrls()
    const photos = new PhotoCollection(urls)
    await photos.add([imageFile('existing.jpg')])
    const failed = imageFile('unreadable.jpg')
    vi.spyOn(failed, 'arrayBuffer').mockRejectedValue(new Error('File read failed'))
    await expect(photos.add([imageFile('first.jpg'), failed])).rejects.toThrow('File read failed')
    expect(photos.entries.map(photo => photo.file.name)).toEqual(['existing.jpg'])
    expect(urls.revokeObjectURL.mock.calls).toEqual([['blob:photo-1']])
    photos.dispose()
    photos.dispose()
    expect(urls.revokeObjectURL.mock.calls).toEqual([['blob:photo-1'], ['blob:photo-0']])
  })

  it('does not create or publish a thumbnail after disposal during a file read', async () => {
    const urls = objectUrls()
    const photos = new PhotoCollection(urls)
    const file = imageFile()
    let resolve!: (value: ArrayBuffer) => void
    vi.spyOn(file, 'arrayBuffer').mockReturnValue(new Promise(done => { resolve = done }))
    const adding = photos.add([file])
    photos.dispose()
    resolve(new ArrayBuffer(0))
    await expect(adding).rejects.toMatchObject({name: 'AbortError'})
    expect(urls.createObjectURL).not.toHaveBeenCalled()
    expect(photos.entries).toEqual([])
  })

  it('rejects an invalid focal edit without corrupting an imported photo', async () => {
    const photos = new PhotoCollection(objectUrls())
    await photos.add([imageFile()])
    for (const value of [NaN, Infinity, -1, 0, 7, 1001]) {
      expect(validPhotoFocal(value)).toBe(false)
      expect(() => photos.setFocal(0, value)).toThrow('Focal equivalent')
    }
    expect(photos.entries[0]?.equivalent).toBe(50)
    photos.setFocal(0, 100)
    expect(photos.entries[0]?.equivalent).toBe(100)
    photos.dispose()
  })

  it('closes a bitmap when cancellation arrives during asynchronous decoding', async () => {
    let resolve!: (bitmap: ImageBitmap) => void
    const close = vi.fn()
    vi.stubGlobal('createImageBitmap', () => new Promise(done => { resolve = done }))
    const abort = new AbortController()
    const decoding = decodePhoto(imageFile(), 50, 960, abort.signal)
    abort.abort()
    resolve({width: 640, height: 480, close} as unknown as ImageBitmap)
    await expect(decoding).rejects.toMatchObject({name: 'AbortError'})
    expect(close).toHaveBeenCalledOnce()
  })

  it('rejects unusable resized aspect ratios before allocating a canvas', async () => {
    const close = vi.fn()
    vi.stubGlobal('createImageBitmap', async () => ({width: 10000, height: 48, close}))
    await expect(decodePhoto(imageFile(), 50)).rejects.toThrow('too small or narrow')
    expect(close).toHaveBeenCalledOnce()
  })
})

describe('photo export and preview separation', () => {
  it('scales the document copy by the full observed extent and leaves PLY untouched', () => {
    const surface = {...tetrahedron, positions: [...tetrahedron.positions, [2, 0, 0]],
      colors: [...tetrahedron.colors, [50, 60, 70]], documentSurface: tetrahedron}
    const before = photoPly(surface)
    const source = photoScadSource(surface, 100, 250000)
    const points = JSON.parse(source.match(/polyhedron\(points=(.*), faces=/)![1]!)
    expect(points[1]).toEqual([50, 0, 0])
    expect(points).toHaveLength(4)
    expect(photoPly(surface)).toBe(before)
    expect(before).toContain('element vertex 5')
    expect(() => photoScadSource(surface, Infinity, 250000)).toThrow('positive width')
    expect(() => photoScadSource(surface, 100, 10)).toThrow('document budget')
    expect(() => photoScadSource({...tetrahedron, triangles: [[0, 1, 2]]}, 100, 250000)).toThrow('valid CAD solid')
  })

  it('reuses preview geometry across mode changes and keeps point colors in separate materials', () => {
    const before = JSON.stringify(tetrahedron)
    const preview = new PhotoPreview(tetrahedron)
    const cloud = preview.meshes('points')
    const surface = preview.meshes('surface')
    expect(preview.meshes('points')).toBe(cloud)
    expect(preview.meshes('surface')).toBe(surface)
    expect(cloud).toHaveLength(4)
    expect(cloud[0]?.color).toEqual([1, 0, 0, 1])
    expect(cloud.reduce((total, mesh) => total + mesh.indices.length / 3, 0)).toBe(16)
    expect([...surface[0]!.indices]).toEqual(tetrahedron.triangles.flat())
    expect(JSON.stringify(tetrahedron)).toBe(before)
  })

  it('bounds displayed points without reducing the exported cloud', () => {
    const positions = Array.from({length: 12001}, (_, i) => [i, i % 3, i % 7])
    const surface = {positions, colors: [], triangles: []}
    const preview = new PhotoPreview(surface)
    const count = preview.meshes('points').reduce((total, mesh) => total + mesh.vertices.length / 24, 0)
    expect(count).toBeLessThanOrEqual(6000)
    expect(photoPly(surface)).toContain('element vertex 12001')
  })
})

describe('photo diagnostics download', () => {
  it('keeps decoder settings and camera refusal reasons when reconstruction fails', () => {
    const input = {image: 0, name: 'canon.jpg', bytes: 1000, equivalent: 35, width: 960, height: 640, focalPixels: 930}
    const diagnostics = {images: [{image: 0, features: 4, candidateCorrespondences: 0, acceptedObservations: 0,
      conflictingMatches: 0, poseAttempts: 0, registered: false, reason: 'insufficient_features'}],
      initialPair: null, seedPairsTested: 0, matchingRequests: 0, computedPairs: 0, bundleRuns: [], warnings: []}
    const json = photoReportJson({settings: {dense: true, resolution: 128, maxImageSide: 960},
      inputs: [input], sparse: diagnostics, dense: null, timings: null})
    const report = JSON.parse(json)
    expect(report.version).toBe(1)
    expect(report.inputs).toEqual([input])
    expect(report.sparse.images[0].reason).toBe('insufficient_features')
    expect(report.dense).toBeNull()
    expect(photoRegistrationReason(report.sparse.images[0].reason, true)).toBe('Мало различимых деталей')
    expect(photoRegistrationReason('future-reason', false)).toBe('future-reason')
  })
})
