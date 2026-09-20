import { beforeEach, describe, expect, it, vi } from 'vitest'
import { readSvgDocument, readSvgDocumentAsync, type SvgOptions } from '../src/services/svgDocument'
import { callGeometryRust, warmGeometryKernel } from '../src/services/geometry/kernel'

vi.mock('../src/services/geometry/kernel', () => ({
  callGeometryRust: vi.fn(),
  warmGeometryKernel: vi.fn(),
}))

beforeEach(() => vi.resetAllMocks())

describe('async SVG document boundary', () => {
  it('rejects invalid options before warming or executing', async () => {
    for (const options of [{ dpi: 0 }, { tolerance: 0 }, { rasterSize: 0 },
      { alphaThreshold: 0 }, { fonts: [new Uint8Array()] }]) {
      await expect(readSvgDocumentAsync('<svg/>', options)).rejects.toThrow()
    }
    expect(warmGeometryKernel).not.toHaveBeenCalled()
    expect(callGeometryRust).not.toHaveBeenCalled()
  })

  it('captures options and font bytes before waiting and uses the same payload as the synchronous API', async () => {
    let ready!: () => void
    vi.mocked(warmGeometryKernel).mockReturnValue(new Promise(resolve => { ready = resolve }))
    const options: SvgOptions = { dpi: 72, fonts: [new Uint8Array([1, 2, 3])] }
    readSvgDocument('<svg/>', options, 'preview', true)
    const expected = vi.mocked(callGeometryRust).mock.calls[0]
    vi.mocked(callGeometryRust).mockClear()
    const result = { normalizedSvg: '<svg/>', regions: [], widthMm: 1, heightMm: 1, warnings: [] }
    vi.mocked(callGeometryRust).mockReturnValue(result)
    const pending = readSvgDocumentAsync('<svg/>', options, 'preview', true)
    expect(callGeometryRust).not.toHaveBeenCalled()
    options.dpi = 300
    options.fonts![0].fill(255)
    options.fonts = []
    ready()
    await expect(pending).resolves.toEqual(result)
    expect(callGeometryRust).toHaveBeenCalledExactlyOnceWith(...expected)
  })

  it('propagates warmup failure without executing and allows a subsequent attempt', async () => {
    vi.mocked(warmGeometryKernel).mockRejectedValueOnce(new Error('compile failed')).mockResolvedValue(undefined)
    await expect(readSvgDocumentAsync('<svg/>')).rejects.toThrow('compile failed')
    expect(callGeometryRust).not.toHaveBeenCalled()
    await readSvgDocumentAsync('<svg/>')
    expect(callGeometryRust).toHaveBeenCalledTimes(1)
  })
})
