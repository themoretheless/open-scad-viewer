import { describe, expect, it } from 'vitest'
import { extractSharpFeatureEdges } from '../src/services/meshTopology'
import {
  computePinchZoomFactor,
  stepInertialOrbitCamera,
  type OrbitCameraState,
} from '../src/services/cameraGestures'
import { inspectStepHeaderMetadata } from '../src/services/cadStepConformance'
import { extractOpenScadDependencies } from '../src/services/openScadProject'
import {
  computeLinearPatternOffsets,
  computeCircularPatternTransforms,
} from '../src/services/directModelingTools'
import {expandLiveBodyPatterns} from '../src/services/liveBodyPattern'
import type {DirectDocument} from '../src/services/directModeling'
import { formatEngineeringSummaryMarkdown } from '../src/mcp/rushGraphReport'

describe('Tier IV Wave 3 Improvements', () => {
  describe('meshTopology extractSharpFeatureEdges', () => {
    it('extracts 4 boundary edges and ignores the coplanar diagonal of a flat quad', () => {
      // Quad in XY plane (z = 0): two triangles (0, 1, 2) and (0, 2, 3) sharing diagonal (0, 2)
      const positions = [
        0, 0, 0,
        10, 0, 0,
        10, 10, 0,
        0, 10, 0,
      ]
      const indices = [
        0, 1, 2,
        0, 2, 3,
      ]
      const edges = extractSharpFeatureEdges({ positions, indices }, 30)
      expect(edges).toHaveLength(4)
      // Verify the shared diagonal (0,0,0) -> (10,10,0) is NOT in the extracted sharp edges
      const hasDiagonal = edges.some(
        ([a, b]) =>
          (a[0] === 0 && a[1] === 0 && b[0] === 10 && b[1] === 10) ||
          (b[0] === 0 && b[1] === 0 && a[0] === 10 && a[1] === 10),
      )
      expect(hasDiagonal).toBe(false)
    })

    it('detects a 90-degree crease edge when creaseAngleDeg is 30° and ignores it at 120°', () => {
      // Two triangles sharing edge (0,0,0)-(10,0,0):
      // Triangle 1 in XY plane (z=0, y>=0), normal (0, 0, +1)
      // Triangle 2 in XZ plane (y=0, z<=0), normal (0, -1, 0) -> 90° dihedral angle
      const positions = [
        0, 0, 0,
        10, 0, 0,
        5, 10, 0,
        5, 0, -10,
      ]
      const indices = [
        0, 1, 2,
        1, 0, 3,
      ]
      // At 30° threshold: 4 boundary edges + 1 sharp 90° crease edge = 5 edges
      const sharpAt30 = extractSharpFeatureEdges({ positions, indices }, 30)
      expect(sharpAt30).toHaveLength(5)

      // At 120° threshold: only the 4 boundary edges are returned
      const sharpAt120 = extractSharpFeatureEdges({ positions, indices }, 120)
      expect(sharpAt120).toHaveLength(4)
    })
  })

  describe('cameraGestures stepInertialOrbitCamera & computePinchZoomFactor', () => {
    it('damps orbit velocity smoothly and clamps pitch/distance bounds', () => {
      const initial: OrbitCameraState = {
        yaw: 0,
        pitch: 0.2,
        dist: 100,
        tx: 0,
        ty: 0,
        tz: 0,
      }
      const step1 = stepInertialOrbitCamera(
        initial,
        { yawRadPerSec: 2.0, pitchRadPerSec: -1.0, zoomPerSec: -0.5 },
        0.05,
        8,
      )
      expect(step1.state.yaw).toBeGreaterThan(0)
      expect(step1.state.pitch).toBeLessThan(0.2)
      expect(step1.state.dist).toBeLessThan(100)
      expect(step1.velocity.yawRadPerSec).toBeLessThan(2.0)
      expect(step1.velocity.yawRadPerSec).toBeGreaterThan(0)
      expect(step1.moving).toBe(true)

      // Large time step brings velocity to rest
      const settled = stepInertialOrbitCamera(
        step1.state,
        step1.velocity,
        5.0,
        8,
      )
      expect(settled.moving).toBe(false)
      expect(settled.velocity).toEqual({
        yawRadPerSec: 0,
        pitchRadPerSec: 0,
        zoomPerSec: 0,
      })
    })

    it('computes bounded pinch-to-zoom distance factors', () => {
      // Spreading fingers from 100px to 200px halves camera distance (factor = 0.5)
      expect(computePinchZoomFactor(100, 200)).toBeCloseTo(0.5, 6)
      // Pinching fingers from 200px to 100px doubles camera distance (factor = 2.0)
      expect(computePinchZoomFactor(200, 100)).toBeCloseTo(2.0, 6)
      // Degenerate or non-positive inputs safely return 1
      expect(computePinchZoomFactor(0, 100)).toBe(1)
      expect(computePinchZoomFactor(Number.NaN, 100)).toBe(1)
    })
  })

  describe('cadStepConformance inspectStepHeaderMetadata', () => {
    it('extracts STEP schema, fileName, author, organization, entity count, and representation flags', () => {
      const sampleStep = [
        'ISO-10303-21;',
        'HEADER;',
        "FILE_DESCRIPTION(('OpenSCAD Viewer B-Rep Export'), '2;1');",
        "FILE_NAME('gearbox-housing.step', '2026-10-21T08:00:00', ('Ada Lovelace'), ('Analytical Mechanics Ltd'), 'polygon-core', 'open-scad-viewer', '');",
        "FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));",
        'ENDSEC;',
        'DATA;',
        "#1 = APPLICATION_CONTEXT('automotive design');",
        "#2 = MANIFOLD_SOLID_BREP('Body1', #3);",
        "#3 = CLOSED_SHELL('', (#4));",
        "#4 = ADVANCED_FACE('', (#5), #6, .T.);",
        'ENDSEC;',
        'END-ISO-10303-21;',
      ].join('\n')

      const meta = inspectStepHeaderMetadata(sampleStep)
      expect(meta.schema).toBe('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }')
      expect(meta.fileName).toBe('gearbox-housing.step')
      expect(meta.author).toBe('Ada Lovelace')
      expect(meta.organization).toBe('Analytical Mechanics Ltd')
      expect(meta.entityCount).toBe(4)
      expect(meta.hasAdvancedBrep).toBe(true)
      expect(meta.hasTessellatedShape).toBe(false)
    })

    it('detects AP242 tessellated shapes and handles empty input gracefully', () => {
      const tessStep = [
        'ISO-10303-21;',
        'HEADER;',
        "FILE_NAME('mesh.stp', '2026-10-21', (''), (''), '', '', '');",
        "FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));",
        'ENDSEC;',
        'DATA;',
        "#10 = TESSELLATED_SHAPE_REPRESENTATION('Tess', (#11), #12);",
        "#11 = TRIANGULATED_FACE('Face1', ...);",
        'ENDSEC;',
      ].join('\n')
      const meta = inspectStepHeaderMetadata(tessStep)
      expect(meta.schema).toBe('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF')
      expect(meta.fileName).toBe('mesh.stp')
      expect(meta.author).toBeNull()
      expect(meta.organization).toBeNull()
      expect(meta.entityCount).toBe(2)
      expect(meta.hasAdvancedBrep).toBe(false)
      expect(meta.hasTessellatedShape).toBe(true)

      expect(inspectStepHeaderMetadata('')).toEqual({
        schema: null,
        fileName: null,
        author: null,
        organization: null,
        entityCount: 0,
        hasAdvancedBrep: false,
        hasTessellatedShape: false,
      })
    })
  })

  describe('openScadProject extractOpenScadDependencies', () => {
    it('extracts include and use directives while ignoring line/block comments and string literals', () => {
      const scadSource = [
        '// include <commented_out_line.scad>',
        'include <MCAD/boxes.scad>',
        'use <utils/fasteners.scad>;',
        '/*',
        '  use <commented_out_block.scad>',
        '  include <also_ignored.scad>',
        '*/',
        'label = "include <inside_string.scad> use <string_use.scad>";',
        'include <MCAD/boxes.scad> // duplicate should be deduplicated',
        'include <config/tolerances.scad>',
        'use <kinematics/joints.scad>',
      ].join('\n')

      const deps = extractOpenScadDependencies(scadSource)
      expect(deps).toEqual({
        includes: ['MCAD/boxes.scad', 'config/tolerances.scad'],
        uses: ['utils/fasteners.scad', 'kinematics/joints.scad'],
      })
    })
  })

  describe('native live body patterns & directModelingTools pattern helpers', () => {
    it('computes 2D grid linear pattern offsets in row-major order', () => {

      const offsets = computeLinearPatternOffsets(3, 2, 15, 25)
      const source:DirectDocument={version:1,sketches:[],bodies:[{id:'seed',name:'Seed',mesh:{positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]},livePattern:{kind:'grid',rows:2,columns:3,spacing:[15,25]}}]}
      const expanded=expandLiveBodyPatterns(source)
      expect(expanded.bodies.slice(1).map(b=>b.instance!.matrix.slice(0,3).map(row=>row[3]))).toEqual(offsets.slice(1))
      expect(offsets).toEqual([
        [0, 0, 0],
        [15, 0, 0],
        [30, 0, 0],
        [0, 25, 0],
        [15, 25, 0],
        [30, 25, 0],
      ])
    })

    it('computes circular pattern transforms for full 360° and partial arcs', () => {
      const full4 = computeCircularPatternTransforms(4, 20, 360)
      expect(full4).toHaveLength(4)
      expect(full4[0]!.offset).toEqual([20, 0, 0])
      expect(full4[0]!.angleRad).toBeCloseTo(0, 6)
      expect(full4[1]!.offset[0]).toBeCloseTo(0, 6)
      expect(full4[1]!.offset[1]).toBeCloseTo(20, 6)
      expect(full4[1]!.angleRad).toBeCloseTo(Math.PI / 2, 6)
      expect(full4[2]!.offset[0]).toBeCloseTo(-20, 6)
      expect(full4[2]!.offset[1]).toBeCloseTo(0, 6)

      // Partial 180° arc with 3 instances -> 0°, 90°, 180°
      const arc3 = computeCircularPatternTransforms(3, 10, 180)
      expect(arc3).toHaveLength(3)
      expect(arc3[0]!.angleRad).toBeCloseTo(0, 6)
      expect(arc3[1]!.angleRad).toBeCloseTo(Math.PI / 2, 6)
      expect(arc3[2]!.angleRad).toBeCloseTo(Math.PI, 6)
    })
  })

  describe('rushGraphReport formatEngineeringSummaryMarkdown', () => {
    it('formats a structured Markdown summary with volume, surface area, and warnings', () => {
      const md = formatEngineeringSummaryMarkdown({
        nodeCount: 12,
        bodyCount: 3,
        totalVolumeMm3: 15_420.5,
        totalSurfaceAreaMm2: 4_850.25,
        warnings: ['Wall thickness below 1.2 mm on boss_2', 'Unconstrained sketch dof=2'],
      })
      expect(md).toContain('# Engineering Summary Report')
      expect(md).toContain('| **Graph Nodes** | 12 |')
      expect(md).toContain('| **Solid Bodies** | 3 |')
      expect(md).toContain('15420.50 mm³ (15.421 cm³)')
      expect(md).toContain('4850.25 mm² (48.50 cm²)')
      expect(md).toContain('## Warnings')
      expect(md).toContain('- ⚠️ Wall thickness below 1.2 mm on boss_2')

      const cleanMd = formatEngineeringSummaryMarkdown({
        nodeCount: 4,
        bodyCount: 1,
      })
      expect(cleanMd).toContain('| **Total Volume** | N/A |')
      expect(cleanMd).toContain('_Status: Clean — 0 warnings._')
    })
  })
})
