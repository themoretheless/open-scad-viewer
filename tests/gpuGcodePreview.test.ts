import { describe, it, expect } from 'vitest'
import {
  referenceFilterGcodeSegments,
  runGpuFilterGcodeSegments,
  GCODE_SEGMENTS_WGSL,
} from '../src/services/gpuGcodePreview'

describe('WebGPU Compute G-Code Toolpath Segment & Layer Filter', () => {
  const sampleMoveRows = new Float64Array([
    // x, y, z, e, feedrateMmS, layerIndex, extrudedFlag
    0, 0, 0.2, 0.0, 60, 0, 0,
    10, 0, 0.2, 0.5, 30, 0, 1,
    10, 10, 0.2, 1.0, 60, 0, 1,
    0, 10, 0.2, 1.0, 120, 0, 0, // travel on layer 0
    0, 10, 0.4, 1.0, 120, 1, 0, // Z hop to layer 1
    20, 10, 0.4, 2.0, 90, 1, 1, // extrusion on layer 1
  ])

  it('filters G-Code segments by layer range and travel/extrusion flags', () => {
    const layer0OnlyExtrusion = referenceFilterGcodeSegments(sampleMoveRows, {
      minLayer: 0,
      maxLayer: 0,
      includeTravel: false,
      includeExtrusion: true,
      minFeedrateMmS: 30,
      maxFeedrateMmS: 90,
    })

    expect(layer0OnlyExtrusion.segmentCount).toBe(2)
    expect(layer0OnlyExtrusion.segments[0]?.p0).toEqual([0, 0, 0.2])
    expect(layer0OnlyExtrusion.segments[0]?.p1).toEqual([10, 0, 0.2])
    expect(layer0OnlyExtrusion.segments[0]?.feedrateNorm).toBeCloseTo(0.0, 5)
    expect(layer0OnlyExtrusion.segments[1]?.feedrateNorm).toBeCloseTo(0.5, 5)
  })

  it('executes runGpuFilterGcodeSegments across all layers', async () => {
    const allSegs = await runGpuFilterGcodeSegments(sampleMoveRows, {
      minLayer: 0,
      maxLayer: 1,
      includeTravel: true,
      includeExtrusion: true,
      minFeedrateMmS: 30,
      maxFeedrateMmS: 120,
    })

    expect(allSegs.segmentCount).toBe(5)
    expect(allSegs.rawSegments.length).toBe(40)
    expect(allSegs.segments[4]?.extruded).toBe(true)
    expect(allSegs.segments[4]?.p1).toEqual([20, 10, 0.4])
  })

  it('declares valid WGSL compute shader with feedrate normalization and atomic segment emission', () => {
    expect(GCODE_SEGMENTS_WGSL).toContain('@compute @workgroup_size(WG)')
    expect(GCODE_SEGMENTS_WGSL).toContain('atomicAdd(&segment_counter[0], 1u)')
    expect(GCODE_SEGMENTS_WGSL).toContain('ROW_STRIDE')
  })
})
