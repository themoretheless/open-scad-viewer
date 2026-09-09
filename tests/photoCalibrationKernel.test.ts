import {describe, expect, it} from 'vitest'
import {PhotogrammetryKernel, type PhotoPixels} from '../src/services/photogrammetryKernel'
import {parsePhotoCalibration, photoCalibrationExampleJson, type PhotoCalibrationGroup} from '../src/services/photoCalibration'

function pixels(group?: PhotoCalibrationGroup): PhotoPixels {
  return {width: 320, height: 240, focal: 250, rgb: new Uint8Array(320 * 240 * 3).fill(128),
    ...(group ? {calibration: {group, sourceWidth: 320, sourceHeight: 240}} : {})}
}

describe('actual generated WASM calibration boundary', () => {
  it('imports mixed calibration groups and returns Rust rectification provenance through MGV1', () => {
    const groups = parsePhotoCalibration(photoCalibrationExampleJson()).groups
    const kernel = new PhotogrammetryKernel()
    try {
      expect(kernel.add(pixels(groups[0]))).toBe(1)
      expect(kernel.add(pixels(groups[1]))).toBe(2)
      expect(kernel.add(pixels())).toBe(3)
      const report = kernel.report()!
      expect(report.calibrations!.map(row => row.mode)).toEqual(['measured-brown', 'measured-brown', 'focal-hint'])
      expect(report.calibrations![0].group).toEqual(groups[0])
      expect(report.calibrations![0].borderPolicy).toBe('fully-valid')
      expect(report.calibrations![0].output.focal).toBeGreaterThan(320)
      expect(report.calibrations![2].output.focal).toBe(250)
      // Uniform images intentionally fail feature reconstruction; measurements survive.
      expect(() => kernel.sparse()).toThrow()
      expect(kernel.report()!.calibrations).toEqual(report.calibrations)
    } finally { kernel.clear() }
    expect(kernel.report()).toBeNull()
  })

  it('rejects group conflicts, wrong source dimensions and oversized metadata transactionally', () => {
    const group = parsePhotoCalibration(photoCalibrationExampleJson()).groups[0]
    const kernel = new PhotogrammetryKernel()
    try {
      kernel.add(pixels(group))
      const before = kernel.report()
      expect(() => kernel.add(pixels({...group, fx: group.fx + 1}))).toThrow('conflicting')
      const wrongSize = pixels({...group, id: 'wrong-size'})
      wrongSize.calibration!.sourceWidth = 640
      expect(() => kernel.add(wrongSize)).toThrow('oriented original')
      expect(() => kernel.add(pixels({...group, id: 'too-large', source: 'x'.repeat(20_000)}))).toThrow('16 KiB')
      expect(kernel.report()).toEqual(before)
      expect(kernel.add(pixels({...group, id: 'recovered'}))).toBe(2)
    } finally { kernel.clear() }
  })
})
