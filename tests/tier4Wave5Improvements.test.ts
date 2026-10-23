import { describe, expect, it } from 'vitest'
import { inspectIgesHeaderMetadata } from '../src/services/cadIges'
import { computeSurfaceGroupAreaSummary } from '../src/services/meshSurfaceGroups'
import {
  isOpenScadBuiltinFunction,
  findClosestOpenScadBuiltinFunction,
} from '../src/services/openScadBuiltinFunctions'
import { generateDrawingTitleBlockSvg } from '../src/services/drawingSheets'
import { describePublicErrorCode, PUBLIC_ERROR_CODES } from '../src/mcp/errorContract'
import { describePublicErrorCode as describePublicErrorCodeFromPublicError } from '../src/mcp/publicError'
import {
  getGpuExecutionTopologyReport,
  getGpuMemoryArchitectureProfile,
} from '../src/renderer/types'
import {
  createAndUploadBufferByArchitecture,
  detectGpuMemoryArchitecture,
} from '../src/services/gpuZeroCopyPipeline'
import { sameTypedArray, sortEdgeDraws, sortOpaqueDraws } from '../src/renderer/drawSorting'
import { isSectionClipped, selectionCandidateKey } from '../src/renderer/pickingController'
import { groupByShadingModel } from '../src/renderer/framePassEncoder'
import { deepEqual, identityValue, truthy, valueToString } from '../src/parser/astDiscovery'
import { compatibilityString } from '../src/parser/expressionEvaluator'
import { sha256, truncateWellFormed } from '../src/mcp/server/contracts'

function padIgesLine(content: string, section: 'S' | 'G' | 'D' | 'P' | 'T', sequence: number): string {
  const paddedContent = content.padEnd(72, ' ').slice(0, 72)
  const paddedSeq = String(sequence).padStart(7, ' ')
  return `${paddedContent}${section}${paddedSeq}`
}

describe('Tier IV Wave 5 Improvements', () => {
  describe('cadIges inspectIgesHeaderMetadata', () => {
    it('parses IGES 5.3 Start, Global, Directory, and Parameter sections for metadata and entity types', () => {
      const igesLines = [
        padIgesLine('OpenSCAD Viewer Direct B-Rep IGES 5.3 Export', 'S', 1),
        padIgesLine('Analytical Mechanics Solid Model', 'S', 2),
        padIgesLine(
          '1H,,1H;,13HTURBINE-ROTOR,17Hturbine_rotor.igs,16Hopen-scad-viewer,',
          'G',
          1,
        ),
        padIgesLine(
          '7H2026.10,32,38,6,308,15,13HTURBINE-ROTOR,1.0,2,2HMM,1,0.08,15H20261021.120000;',
          'G',
          2,
        ),
        // Directory Entry 1: Entity 128 (Rational B-Spline Surface)
        padIgesLine('     128       1       0       0       1       0       0       000000001', 'D', 1),
        padIgesLine('     128       0       0       1       0                        Surface1', 'D', 2),
        // Directory Entry 2: Entity 144 (Trimmed Parametric Surface)
        padIgesLine('     144       2       0       0       1       0       0       000000001', 'D', 3),
        padIgesLine('     144       0       0       1       0                        Trimmed1', 'D', 4),
        // Parameter Data
        padIgesLine('128,1,1,1,1,0,0,1,0,0,0.,0.,1.,1.,0.,0.,1.,1.,1.,1.,1.,1.;             1', 'P', 1),
        padIgesLine('144,1,0,0,0;                                                           3', 'P', 2),
        padIgesLine('S      2G      2D      4P      2                                        ', 'T', 1),
      ]
      const igesText = igesLines.join('\n')

      const meta = inspectIgesHeaderMetadata(igesText)
      expect(meta.startComments).toEqual([
        'OpenSCAD Viewer Direct B-Rep IGES 5.3 Export',
        'Analytical Mechanics Solid Model',
      ])
      expect(meta.globalDelimiter).toBe(',')
      expect(meta.recordDelimiter).toBe(';')
      expect(meta.productId).toBe('TURBINE-ROTOR')
      expect(meta.fileName).toBe('turbine_rotor.igs')
      expect(meta.unitsCode).toBe(2)
      expect(meta.unitsName).toBe('MM')
      expect(meta.directoryEntryCount).toBe(2)
      expect(meta.hasNurbsSurface128).toBe(true)
      expect(meta.hasTrimmedSurface144).toBe(true)
    })

    it('handles empty input and custom IGES delimiters gracefully', () => {
      expect(inspectIgesHeaderMetadata('')).toEqual({
        startComments: [],
        globalDelimiter: ',',
        recordDelimiter: ';',
        productId: null,
        fileName: null,
        unitsCode: null,
        unitsName: null,
        directoryEntryCount: 0,
        hasNurbsSurface128: false,
        hasTrimmedSurface144: false,
      })

      const inchIges = [
        padIgesLine('Inch Part', 'S', 1),
        padIgesLine(',,7Hpart_in,11Hpart_in.igs,3Hsys,3Hver,32,38,6,308,15,7Hpart_in,1.0,1,,;', 'G', 1),
        padIgesLine('     110       1       0       0       1       0       0       000000001', 'D', 1),
        padIgesLine('     110       0       0       1       0                           Line1', 'D', 2),
      ].join('\n')
      const meta = inspectIgesHeaderMetadata(inchIges)
      expect(meta.productId).toBe('part_in')
      expect(meta.fileName).toBe('part_in.igs')
      expect(meta.unitsCode).toBe(1)
      expect(meta.unitsName).toBe('IN')
      expect(meta.directoryEntryCount).toBe(1)
      expect(meta.hasNurbsSurface128).toBe(false)
      expect(meta.hasTrimmedSurface144).toBe(false)
    })
  })

  describe('meshSurfaceGroups computeSurfaceGroupAreaSummary', () => {
    it('groups triangles by surfaceId and computes exact area and area-weighted unit normals', () => {
      // Surface 10: a 10x20 rectangle in XY plane (z = 0), normal = (0, 0, +1), area = 200 mm²
      // Surface 20: a right triangle in YZ plane (x = 0) with legs 6 and 8, normal = (+1, 0, 0), area = 24 mm²
      const positions = new Float32Array([
        // Surface 10 vertices (0..3)
        0, 0, 0,
        10, 0, 0,
        10, 20, 0,
        0, 20, 0,
        // Surface 20 vertices (4..6)
        0, 0, 0,
        0, 6, 0,
        0, 0, 8,
      ])
      const indices = new Uint32Array([
        0, 1, 2,
        0, 2, 3,
        4, 5, 6,
      ])
      const surfaceIds = new Uint32Array([10, 10, 20])

      const summary = computeSurfaceGroupAreaSummary({ positions, indices }, surfaceIds)
      expect(summary).toHaveLength(2)

      expect(summary[0]!.surfaceId).toBe(10)
      expect(summary[0]!.triangleCount).toBe(2)
      expect(summary[0]!.areaMm2).toBeCloseTo(200, 6)
      expect(summary[0]!.averageNormal[0]).toBeCloseTo(0, 6)
      expect(summary[0]!.averageNormal[1]).toBeCloseTo(0, 6)
      expect(summary[0]!.averageNormal[2]).toBeCloseTo(1, 6)

      expect(summary[1]!.surfaceId).toBe(20)
      expect(summary[1]!.triangleCount).toBe(1)
      expect(summary[1]!.areaMm2).toBeCloseTo(24, 6)
      expect(summary[1]!.averageNormal[0]).toBeCloseTo(1, 6)
      expect(summary[1]!.averageNormal[1]).toBeCloseTo(0, 6)
      expect(summary[1]!.averageNormal[2]).toBeCloseTo(0, 6)
    })
  })

  describe('openScadBuiltinFunctions isOpenScadBuiltinFunction & findClosestOpenScadBuiltinFunction', () => {
    it('performs O(1) lookup for OpenSCAD built-in functions and primitives', () => {
      expect(isOpenScadBuiltinFunction('sin')).toBe(true)
      expect(isOpenScadBuiltinFunction('concat')).toBe(true)
      expect(isOpenScadBuiltinFunction('search')).toBe(true)
      expect(isOpenScadBuiltinFunction('cylinder')).toBe(true)
      expect(isOpenScadBuiltinFunction('translate')).toBe(true)
      expect(isOpenScadBuiltinFunction('my_custom_helper')).toBe(false)
      expect(isOpenScadBuiltinFunction('')).toBe(false)
    })

    it('suggests closest built-in names using Levenshtein edit distance', () => {
      expect(findClosestOpenScadBuiltinFunction('cylindr')).toBe('cylinder')
      expect(findClosestOpenScadBuiltinFunction('translat')).toBe('translate')
      expect(findClosestOpenScadBuiltinFunction('conct')).toBe('concat')
      expect(findClosestOpenScadBuiltinFunction('serch')).toBe('search')
      expect(findClosestOpenScadBuiltinFunction('is_undf')).toBe('is_undef')
      expect(findClosestOpenScadBuiltinFunction('completely_unrelated_identifier_xyz', 2)).toBeNull()
    })
  })

  describe('drawingSheets generateDrawingTitleBlockSvg', () => {
    it('generates a vector SVG sheet frame and GOST/ISO title block stamp', () => {
      const svg = generateDrawingTitleBlockSvg({
        format: 'A3',
        title: 'Planetary Carrier Housing & Flange',
        partNumber: 'ASM-2026-042',
        material: 'AlSi10Mg-T6',
        scale: '2:1',
        author: 'N. Tesla',
        dateIso: '2026-10-21',
      })

      expect(svg).toContain('<svg xmlns="http://www.w3.org/2000/svg" width="420mm" height="297mm"')
      expect(svg).toContain('data-role="sheet-frame"')
      expect(svg).toContain('data-role="title-block"')
      expect(svg).toContain('Planetary Carrier Housing &amp; Flange')
      expect(svg).toContain('ASM-2026-042')
      expect(svg).toContain('AlSi10Mg-T6')
      expect(svg).toContain('2:1')
      expect(svg).toContain('N. Tesla')
      expect(svg).toContain('2026-10-21')
      expect(svg).toContain('A3')
    })
  })

  describe('mcp/errorContract describePublicErrorCode', () => {
    it('provides structured severity, summary, and remediation hints for all public error codes', () => {
      expect(describePublicErrorCodeFromPublicError).toBe(describePublicErrorCode)

      for (const code of PUBLIC_ERROR_CODES) {
        const desc = describePublicErrorCode(code)
        expect(desc.code).toBe(code)
        expect(['recoverable', 'fatal']).toContain(desc.severity)
        expect(desc.summary.length).toBeGreaterThan(15)
        expect(desc.remediationHint.length).toBeGreaterThan(15)
      }

      const syntaxDesc = describePublicErrorCode('source_syntax_error')
      expect(syntaxDesc.severity).toBe('recoverable')
      expect(syntaxDesc.remediationHint).toContain('line/column')

      const engineDesc = describePublicErrorCode('engine_unavailable')
      expect(engineDesc.severity).toBe('fatal')

      const unknownDesc = describePublicErrorCode('non_existent_code')
      expect(unknownDesc.severity).toBe('fatal')
      expect(unknownDesc.code).toBe('non_existent_code')
    })
  })

  describe('UMA vs dUMA GPU memory architecture separation', () => {
    it('classifies Apple Silicon / Metal as UMA and CUDA / WebGPU as dUMA with distinct upload/readback profiles', () => {
      const metalReport = getGpuExecutionTopologyReport('metal')
      expect(metalReport.memoryArchitecture).toBe('uma')
      expect(metalReport.memoryProfile.unifiedPhysicalRam).toBe(true)
      expect(metalReport.memoryProfile.zeroCopyPrimaryUpload).toBe(true)
      expect(metalReport.memoryProfile.requiresExplicitStagingRing).toBe(false)
      expect(metalReport.memoryProfile.preferredUploadStrategy).toBe('direct_mapped_primary_buffer')

      const cudaReport = getGpuExecutionTopologyReport('cuda')
      expect(cudaReport.memoryArchitecture).toBe('duma')
      expect(cudaReport.memoryProfile.unifiedPhysicalRam).toBe(false)
      expect(cudaReport.memoryProfile.zeroCopyPrimaryUpload).toBe(false)
      expect(cudaReport.memoryProfile.requiresExplicitStagingRing).toBe(true)
      expect(cudaReport.memoryProfile.preferredUploadStrategy).toBe('staging_ring_to_device_local_vram')

      expect(getGpuMemoryArchitectureProfile('uma').architecture).toBe('uma')
      expect(getGpuMemoryArchitectureProfile('duma').architecture).toBe('duma')

      expect(detectGpuMemoryArchitecture({ vendor: 'Apple', architecture: 'metal-3' }, null)).toBe('uma')
      expect(
        detectGpuMemoryArchitecture(
          { vendor: 'NVIDIA', architecture: 'blackwell' },
          { has: (f) => f === 'mappable-primary-buffers' },
        ),
      ).toBe('uma')
      expect(detectGpuMemoryArchitecture({ vendor: 'NVIDIA', architecture: 'blackwell' }, null)).toBe('duma')
    })

    it('uses direct mappedAtCreation with MAP_WRITE on UMA vs COPY_DST + queue.writeBuffer on dUMA', () => {
      const payload = new Float32Array([1, 2, 3, 4])
      let umaCreatedDesc: { size: number; usage: number; mappedAtCreation?: boolean } | null = null
      let umaUnmapped = false
      const umaMappedBacking = new ArrayBuffer(16)
      const mockUmaDevice = {
        features: { has: (f: string) => f === 'mappable-primary-buffers' },
        createBuffer: (desc: { size: number; usage: number; mappedAtCreation?: boolean }) => {
          umaCreatedDesc = desc
          return {
            getMappedRange: () => umaMappedBacking,
            unmap: () => { umaUnmapped = true },
          }
        },
        queue: {
          writeBuffer: () => {
            throw new Error('UMA path should not use queue.writeBuffer')
          },
        },
      } as unknown as GPUDevice

      createAndUploadBufferByArchitecture(mockUmaDevice, payload, 0x0080 /* STORAGE */, 'uma')
      expect(umaCreatedDesc).not.toBeNull()
      expect(umaCreatedDesc!.mappedAtCreation).toBe(true)
      expect((umaCreatedDesc!.usage & 0x0002) !== 0).toBe(true) // MAP_WRITE added on UMA
      expect(umaUnmapped).toBe(true)
      expect(Array.from(new Float32Array(umaMappedBacking))).toEqual([1, 2, 3, 4])

      let dumaCreatedDesc: { size: number; usage: number; mappedAtCreation?: boolean } | null = null
      let dumaQueueWrittenBytes = 0
      const mockDumaDevice = {
        features: { has: () => false },
        createBuffer: (desc: { size: number; usage: number; mappedAtCreation?: boolean }) => {
          dumaCreatedDesc = desc
          return {}
        },
        queue: {
          writeBuffer: (_buf: unknown, _offset: number, _data: ArrayBuffer, _dataOffset: number, size: number) => {
            dumaQueueWrittenBytes = size
          },
        },
      } as unknown as GPUDevice

      createAndUploadBufferByArchitecture(mockDumaDevice, payload, 0x0080 /* STORAGE */, 'duma')
      expect(dumaCreatedDesc).not.toBeNull()
      expect(dumaCreatedDesc!.mappedAtCreation).toBe(false)
      expect((dumaCreatedDesc!.usage & 0x0008) !== 0).toBe(true) // COPY_DST added on dUMA
      expect(dumaQueueWrittenBytes).toBe(16)
    })
  })

  describe('Decomposed renderer, parser, and MCP server micro-modules', () => {
    it('verifies renderer drawSorting, pickingController, and framePassEncoder helpers', () => {
      const a = new Float32Array([1, 2, 3])
      const b = new Float32Array([1, 2, 3])
      const c = new Float32Array([1, 2, 4])
      expect(sameTypedArray(a, b)).toBe(true)
      expect(sameTypedArray(a, c)).toBe(false)

      const vb1 = { label: 'vb1' } as GPUBuffer
      const vb2 = { label: 'vb2' } as GPUBuffer
      const ib1 = { label: 'ib1' } as GPUBuffer
      const bg1 = { label: 'bg1' } as GPUBindGroup
      const bg2 = { label: 'bg2' } as GPUBindGroup
      const draws = [
        { vb: vb2, ib: ib1, edgeIB: ib1, bg: bg2, shadingModel: 'pbr', ic: 12, edgeIC: 8 },
        { vb: vb1, ib: ib1, edgeIB: ib1, bg: bg1, shadingModel: 'phong', ic: 12, edgeIC: 8 },
        { vb: vb2, ib: ib1, edgeIB: ib1, bg: bg1, shadingModel: 'phong', ic: 12, edgeIC: 8 },
      ] as any[]
      sortOpaqueDraws(draws)
      // 'pbr' < 'phong' lexicographically, and within 'phong' the two vb2/vb1 meshes are grouped deterministically
      expect(draws[0].shadingModel).toBe('pbr')
      expect(draws[1].shadingModel).toBe('phong')
      expect(draws[2].shadingModel).toBe('phong')
      sortEdgeDraws(draws)
      // After sortEdgeDraws, both vb2 meshes are adjacent
      expect(draws[0].vb === draws[1].vb || draws[1].vb === draws[2].vb).toBe(true)

      const groups = groupByShadingModel(draws)
      expect(groups).toHaveLength(2)

      expect(isSectionClipped([0, 0, -2], true, [0, 0, 1], 0)).toBe(true)
      expect(isSectionClipped([0, 0, 2], true, [0, 0, 1], 0)).toBe(false)
      expect(isSectionClipped([0, 0, -2], false, [0, 0, 1], 0)).toBe(false)

      const sampleHit = {
        meshIndex: 2,
        triangleIndex: 7,
        faceId: 3,
        point: [1, 2, 3] as [number, number, number],
        normal: [0, 0, 1] as [number, number, number],
        barycentric: [1, 0, 0] as [number, number, number],
        source: null,
        backside: false,
      }
      expect(selectionCandidateKey(sampleHit, 'object')).toBe('o:2')
      expect(selectionCandidateKey(sampleHit, 'face')).toBe('f:2:3')
      expect(selectionCandidateKey(sampleHit, 'point')).toContain('p:2:')
    })

    it('verifies parser astDiscovery/expressionEvaluator and MCP server contract helpers', () => {
      expect(valueToString([1, true, undefined])).toBe('[1, true, undef]')
      expect(identityValue('abc')).toBe('"abc"')
      expect(truthy(0)).toBe(false)
      expect(truthy([])).toBe(false)
      expect(truthy([0])).toBe(true)
      expect(deepEqual([1, [2, 3]], [1, [2, 3]])).toBe(true)
      expect(deepEqual([1, [2, 3]], [1, [2, 4]])).toBe(false)
      expect(compatibilityString(undefined)).toBe('')
      expect(compatibilityString(42)).toBe('42')

      expect(sha256('open-scad-viewer')).toHaveLength(64)
      // Surrogate pair safety: \uD83D\uDE80 (🚀) at boundary should not leave an orphan high surrogate
      const rocket = 'abc\uD83D\uDE80def'
      expect(truncateWellFormed(rocket, 4)).toBe('abc')
      expect(truncateWellFormed(rocket, 5)).toBe('abc\uD83D\uDE80')
    })
  })
})
