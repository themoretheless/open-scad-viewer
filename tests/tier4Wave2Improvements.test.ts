import { describe, expect, it } from 'vitest'
import {
  SOLID_ENGINEERING_MATERIALS,
  estimateSolidMassGrams,
  solidMaterialRgb,
} from '../src/services/solidMaterial'
import { inspectMeshManifoldDiagnostics } from '../src/services/meshInspection'
import { computeGcodeLayerSummary } from '../src/services/gcodePreviewGeometry'
import {
  exportCustomizerPresetsJson,
  importCustomizerPresetsJson,
  applyCustomizerPresetToSource,
} from '../src/services/scadCustomizer'
import {
  DRAWING_SHEET_FORMATS,
  formatDrawingDimensionWithTolerance,
} from '../src/services/drawingSheets'
import { computeAdaptiveGridStepMm } from '../src/components/ModelingGridControls.vue'
import { computeSteppedOrbitAngles } from '../src/components/ViewCube.vue'
import { formatInterferenceMarkdownReport } from '../src/mcp/rushGraphInterference'

const IDENTITY_TRANSFORM = new Float32Array([
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
])

describe('Tier IV Wave 2 Improvements', () => {
  describe('solidMaterial engineering catalog & mass estimator', () => {
    it('contains all 9 standard engineering materials with valid physical properties', () => {
      const expectedKeys = [
        'pla',
        'petg',
        'abs',
        'nylon_pa12',
        'aluminum_6061',
        'steel_304',
        'brass_c360',
        'titanium_ti6al4v',
        'carbon_fiber_composite',
      ]
      for (const key of expectedKeys) {
        const mat = SOLID_ENGINEERING_MATERIALS[key]
        expect(mat).toBeDefined()
        expect(mat!.densityGPerCm3).toBeGreaterThan(0)
        expect(mat!.youngsModulusGpa).toBeGreaterThan(0)
        expect(mat!.yieldStrengthMpa).toBeGreaterThan(0)
        expect(mat!.thermalConductivityWPerMK).toBeGreaterThan(0)
        expect(mat!.defaultHexColor).toMatch(/^#[0-9a-f]{6}$/i)
      }
      expect(solidMaterialRgb('#ff8000')[0]).toBeCloseTo(1, 5)
    })

    it('estimates solid mass in grams from volume in mm^3 and material id or name', () => {
      // 1000 mm^3 = 1 cm^3 -> mass in grams equals densityGPerCm3
      expect(estimateSolidMassGrams(1000, 'pla')).toBeCloseTo(1.24, 5)
      expect(estimateSolidMassGrams(10_000, 'Aluminum 6061')).toBeCloseTo(27.0, 5)
      expect(estimateSolidMassGrams(2000, 'steel_304')).toBeCloseTo(16.0, 5)
      expect(estimateSolidMassGrams(0, 'titanium_ti6al4v')).toBe(0)
      expect(() => estimateSolidMassGrams(-10, 'pla')).toThrow(RangeError)
      expect(() => estimateSolidMassGrams(1000, 'unobtainium')).toThrow(/Unknown engineering material/)
    })
  })

  describe('meshInspection manifold diagnostics', () => {
    it('detects a watertight tetrahedron with Euler characteristic V - E + F = 2', () => {
      // 4 vertices, 4 triangles, stride = 6 (x,y,z,nx,ny,nz)
      const vertices = new Float32Array([
        0, 0, 0, 0, 0, -1,
        10, 0, 0, 0, 0, -1,
        0, 10, 0, 0, 0, -1,
        0, 0, 10, 0, 0, 1,
      ])
      const indices = new Uint32Array([
        0, 2, 1,
        0, 1, 3,
        1, 2, 3,
        2, 0, 3,
      ])
      const diag = inspectMeshManifoldDiagnostics({
        vertices,
        indices,
        transform: IDENTITY_TRANSFORM,
      })
      expect(diag.uniqueVertexCount).toBe(4)
      expect(diag.uniqueEdgeCount).toBe(6)
      expect(diag.validFaceCount).toBe(4)
      expect(diag.degenerateTriangleCount).toBe(0)
      expect(diag.boundaryEdgeCount).toBe(0)
      expect(diag.nonManifoldEdgeCount).toBe(0)
      expect(diag.isWatertight).toBe(true)
      expect(diag.eulerCharacteristic).toBe(2)
    })

    it('detects boundary edges, degenerate triangles, and non-manifold edges', () => {
      const vertices = new Float32Array([
        0, 0, 0, 0, 0, 1,
        10, 0, 0, 0, 0, 1,
        0, 10, 0, 0, 0, 1,
        0, 0, 10, 0, 0, 1,
        0, -10, 0, 0, 0, 1,
      ])
      // 3 faces sharing edge (0,1) -> non-manifold edge; plus 1 collinear/zero-area triangle (0, 0, 1)
      const indices = new Uint32Array([
        0, 1, 2,
        0, 1, 3,
        0, 1, 4,
        0, 0, 1,
      ])
      const diag = inspectMeshManifoldDiagnostics({
        vertices,
        indices,
        transform: IDENTITY_TRANSFORM,
      })
      expect(diag.degenerateTriangleCount).toBe(1)
      expect(diag.nonManifoldEdgeCount).toBe(1)
      expect(diag.boundaryEdgeCount).toBe(6)
      expect(diag.isWatertight).toBe(false)
    })
  })

  describe('gcodePreviewGeometry computeGcodeLayerSummary', () => {
    it('computes print vs travel distances, ratio, and bounds', () => {
      expect(computeGcodeLayerSummary([])).toEqual({
        layerCount: 0,
        totalMoves: 0,
        extrusionMoves: 0,
        travelMoves: 0,
        printDistanceMm: 0,
        travelDistanceMm: 0,
        travelToExtrusionRatio: 0,
        bounds: null,
      })

      const summary = computeGcodeLayerSummary([
        { x: 0, y: 0, z: 0.2, e: 0, extruded: false, feedrateMmS: 120, layerIndex: 0 },
        { x: 10, y: 0, z: 0.2, e: 0.5, extruded: true, feedrateMmS: 40, layerIndex: 0 },
        { x: 10, y: 20, z: 0.2, e: 1.5, extruded: true, feedrateMmS: 40, layerIndex: 0 },
        { x: 10, y: 20, z: 0.4, e: 1.5, extruded: false, feedrateMmS: 120, layerIndex: 1 },
      ])
      expect(summary.layerCount).toBe(2)
      expect(summary.totalMoves).toBe(4)
      expect(summary.extrusionMoves).toBe(2)
      expect(summary.travelMoves).toBe(2)
      expect(summary.printDistanceMm).toBeCloseTo(30, 5)
      expect(summary.travelDistanceMm).toBeCloseTo(0.2, 5)
      expect(summary.travelToExtrusionRatio).toBeCloseTo(0.2 / 30, 5)
      expect(summary.bounds).toEqual({
        min: [0, 0, 0.2],
        max: [10, 20, 0.4],
      })
    })
  })

  describe('scadCustomizer preset export, import, and source application', () => {
    it('round-trips presets JSON and applies presets to OpenSCAD source', () => {
      const presets = {
        Enclosure_Small: { width: 40, wall: 1.6, vented: false, label: 'mini' },
        Enclosure_Large: { width: 120, wall: 2.4, vented: true, label: 'pro' },
      }
      const json = exportCustomizerPresetsJson(presets)
      const imported = importCustomizerPresetsJson(json)
      expect(imported).toEqual(presets)

      const source = [
        'width = 60; // [20:200]',
        'wall = 2.0;',
        'vented = true;',
        'label = "default";',
        'cube([width, width, wall]);',
      ].join('\n')

      const updated = applyCustomizerPresetToSource(source, imported.Enclosure_Small!)
      expect(updated).toContain('width = 40;')
      expect(updated).toContain('wall = 1.6;')
      expect(updated).toContain('vented = false;')
      expect(updated).toContain('label = "mini";')
    })
  })

  describe('drawingSheets formats & GD&T tolerance callouts', () => {
    it('exposes ISO A4-A0 sheet formats and formats dimension tolerances', () => {
      expect(DRAWING_SHEET_FORMATS.A4.widthMm).toBe(297)
      expect(DRAWING_SHEET_FORMATS.A0.widthMm).toBe(1189)
      expect(DRAWING_SHEET_FORMATS.A3.marginLeftMm).toBe(20)

      expect(
        formatDrawingDimensionWithTolerance(25, {
          prefix: '⌀',
          fitCode: 'H7',
          symmetricToleranceMm: 0.02,
        }),
      ).toBe('⌀25.00 H7 ±0.02 mm')

      expect(
        formatDrawingDimensionWithTolerance(12.5, {
          upperDeviationMm: 0.05,
          lowerDeviationMm: -0.02,
          decimals: 2,
        }),
      ).toBe('12.50 (+0.05/-0.02) mm')
    })
  })

  describe('ModelingGridControls & ViewCube helpers', () => {
    it('computes adaptive grid step sizes from camera zoom distance', () => {
      expect(computeAdaptiveGridStepMm(12)).toEqual({
        majorStepMm: 1,
        minorStepMm: 0.1,
        label: '1 mm / 0.1 mm',
      })
      expect(computeAdaptiveGridStepMm(120)).toEqual({
        majorStepMm: 10,
        minorStepMm: 1,
        label: '10 mm / 1 mm',
      })
      expect(computeAdaptiveGridStepMm(800)).toEqual({
        majorStepMm: 100,
        minorStepMm: 10,
        label: '100 mm / 10 mm',
      })
    })

    it('computes 15° and 90° stepped orbit angles with clamping', () => {
      const step15 = computeSteppedOrbitAngles(0, 0, 'right', 15)
      expect(step15.yaw).toBeCloseTo((15 * Math.PI) / 180, 6)
      expect(step15.pitch).toBeCloseTo(0, 6)

      const step90Up = computeSteppedOrbitAngles(0, 0.5, 'up', 90)
      expect(step90Up.pitch).toBeCloseTo(Math.PI / 2, 6)
    })
  })

  describe('rushGraphInterference Markdown report formatting', () => {
    it('renders a Markdown table of colliding and clear assembly component pairs', () => {
      const md = formatInterferenceMarkdownReport({
        document_sha256: 'abc123def456',
        scope: 'current joint positions only',
        volume_tolerance_mm3: 1e-6,
        status: 'overlap',
        pairs: [
          {
            a: 'base/rotor',
            b: 'base/stator',
            status: 'overlap',
            intersection_volume_mm3: 14.25,
            method: 'manifold_intersection',
          },
          {
            a: 'base/rotor',
            b: 'base/cover',
            status: 'no_volume_overlap',
            intersection_volume_mm3: 0,
            min_clearance_mm: 1.75,
            method: 'disjoint_or_touching_bounds',
          },
        ],
      })
      expect(md).toContain('# Assembly Interference Report')
      expect(md).toContain('1 colliding, 1 clear, 0 unknown')
      expect(md).toContain('| `base/rotor` | `base/stator` | **overlap** | 14.2500 | 0.000 (collision) | `manifold_intersection` |')
      expect(md).toContain('| `base/rotor` | `base/cover` | **no_volume_overlap** | 0.0000 | 1.750 | `disjoint_or_touching_bounds` |')
    })
  })
})
