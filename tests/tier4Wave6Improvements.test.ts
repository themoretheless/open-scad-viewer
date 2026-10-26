import { describe, expect, it } from 'vitest'
import { evaluateIso286HoleBasisFit } from '../src/services/drawingSheets'
import { computeMeshTriangleQualitySummary } from '../src/services/meshInspection'
import { auditGcodePrintability } from '../src/services/gcodePreviewGeometry'
import { evaluateMechanismMobility3d } from '../src/services/cadJoints'

describe('Tier IV Wave 6 Improvements', () => {
  describe('drawingSheets evaluateIso286HoleBasisFit', () => {
    it('evaluates H7/g6 precision sliding clearance fit for a 25 mm nominal diameter', () => {
      // 25 mm lies in the (18, 30] mm step: D = sqrt(18 * 30) = sqrt(540) ≈ 23.2379 mm
      const fit = evaluateIso286HoleBasisFit(25, 'H7', 'g6')
      expect(fit.nominalDiaMm).toBe(25)
      expect(fit.geometricMeanDiaMm).toBeCloseTo(Math.sqrt(18 * 30), 6)
      expect(fit.holeZone).toBe('H7')
      expect(fit.shaftZone).toBe('g6')
      expect(fit.holeLowerDeviationMm).toBe(0)
      // H7 tolerance around 21 um -> 0.021 mm
      expect(fit.holeUpperDeviationMm).toBeCloseTo(0.0209, 3)
      // g6 upper deviation es < 0 (~ -0.007 mm)
      expect(fit.shaftUpperDeviationMm).toBeLessThan(0)
      expect(fit.shaftLowerDeviationMm).toBeLessThan(fit.shaftUpperDeviationMm)
      expect(fit.minClearanceMm).toBeGreaterThan(0)
      expect(fit.maxClearanceMm).toBeGreaterThan(fit.minClearanceMm)
      expect(fit.fitType).toBe('clearance')
    })

    it('classifies h6 as clearance, k6/n6 as transition, and p6/s6 as interference against H7', () => {
      const h6 = evaluateIso286HoleBasisFit(40, 'H7', 'h6')
      expect(h6.fitType).toBe('clearance')
      expect(h6.minClearanceMm).toBeCloseTo(0, 8)
      expect(h6.maxClearanceMm).toBeGreaterThan(0)

      const f7 = evaluateIso286HoleBasisFit(40, 'H8', 'f7')
      expect(f7.fitType).toBe('clearance')
      expect(f7.minClearanceMm).toBeGreaterThan(0.02)

      const k6 = evaluateIso286HoleBasisFit(40, 'H7', 'k6')
      expect(k6.fitType).toBe('transition')
      expect(k6.minClearanceMm).toBeLessThan(0)
      expect(k6.maxClearanceMm).toBeGreaterThan(0)

      const n6 = evaluateIso286HoleBasisFit(40, 'H7', 'n6')
      expect(n6.fitType).toBe('transition')
      expect(n6.minClearanceMm).toBeLessThan(0)
      expect(n6.maxClearanceMm).toBeGreaterThan(0)

      const p6 = evaluateIso286HoleBasisFit(40, 'H7', 'p6')
      expect(p6.fitType).toBe('interference')
      expect(p6.maxClearanceMm).toBeLessThan(0)
      expect(p6.minClearanceMm).toBeLessThan(p6.maxClearanceMm)

      const s6 = evaluateIso286HoleBasisFit(40, 'H7', 's6')
      expect(s6.fitType).toBe('interference')
      expect(s6.maxClearanceMm).toBeLessThan(p6.maxClearanceMm)
    })

    it('rejects out-of-range nominal diameters', () => {
      expect(() => evaluateIso286HoleBasisFit(0, 'H7', 'h6')).toThrow(RangeError)
      expect(() => evaluateIso286HoleBasisFit(-10, 'H7', 'h6')).toThrow(RangeError)
      expect(() => evaluateIso286HoleBasisFit(501, 'H9', 'g6')).toThrow(RangeError)
    })
  })

  describe('meshInspection computeMeshTriangleQualitySummary', () => {
    it('normalizes an equilateral triangle to aspect ratio 1.0 and computes exact area', () => {
      const side = 10
      const height = (side * Math.sqrt(3)) / 2
      const vertices = new Float32Array([
        0, 0, 0,
        side, 0, 0,
        side / 2, height, 0,
      ])
      const indices = new Uint32Array([0, 1, 2])

      const quality = computeMeshTriangleQualitySummary({ vertices, indices })
      expect(quality.triangleCount).toBe(1)
      expect(quality.degenerateTriangleCount).toBe(0)
      expect(quality.minAspectRatio).toBeCloseTo(1.0, 5)
      expect(quality.maxAspectRatio).toBeCloseTo(1.0, 5)
      expect(quality.meanAspectRatio).toBeCloseTo(1.0, 5)
      const expectedArea = 0.5 * side * height
      expect(quality.minAreaMm2).toBeCloseTo(expectedArea, 4)
      expect(quality.maxAreaMm2).toBeCloseTo(expectedArea, 4)
      expect(quality.totalAreaMm2).toBeCloseTo(expectedArea, 4)
    })

    it('detects high aspect ratio needle triangles and collinear degenerate triangles', () => {
      // Triangle 0: Equilateral side 2 (area = sqrt(3) ≈ 1.73205, aspectRatio = 1.0)
      // Triangle 1: Right triangle base 20, height 1 (hypotenuse = sqrt(401), area = 10)
      // Triangle 2: Collinear degenerate triangle (0,0,0) -> (5,0,0) -> (10,0,0)
      const hEq = Math.sqrt(3)
      const vertices = [
        0, 0, 0,
        2, 0, 0,
        1, hEq, 0,
        0, 0, 0,
        20, 0, 0,
        0, 1, 0,
        0, 0, 0,
        5, 0, 0,
        10, 0, 0,
      ]
      const indices = [
        0, 1, 2,
        3, 4, 5,
        6, 7, 8,
      ]

      const summary = computeMeshTriangleQualitySummary({ vertices, indices })
      expect(summary.triangleCount).toBe(3)
      expect(summary.degenerateTriangleCount).toBe(1)
      expect(summary.minAspectRatio).toBeCloseTo(1.0, 5)
      // For Triangle 1: maxEdgeSq = 401, doubleArea = 20 -> aspectRatio = (sqrt(3)/2) * 401 / 20 ≈ 17.3638
      expect(summary.maxAspectRatio).toBeCloseTo((Math.sqrt(3) / 2) * (401 / 20), 4)
      expect(summary.meanAspectRatio).toBeCloseTo(0.5 * (1.0 + summary.maxAspectRatio), 4)
      expect(summary.minAreaMm2).toBeCloseTo(hEq, 5)
      expect(summary.maxAreaMm2).toBeCloseTo(10, 5)
      expect(summary.totalAreaMm2).toBeCloseTo(hEq + 10, 5)
    })
  })

  describe('gcodePreviewGeometry auditGcodePrintability', () => {
    it('tracks retractions, unretractions, Z-hops, travel-to-extrude ratio, and peak feedrate', () => {
      const gcode = [
        '; Start G-Code',
        'G90 ; absolute positioning',
        'M82 ; absolute extrusion',
        'G92 X0 Y0 Z0.2 E0',
        'G1 X50 Y0 E2.5 F1800 ; print 50mm segment',
        'G1 X50 Y50 E5.0 F1800 ; print 50mm segment (total extrude = 100mm)',
        'G1 E4.2 F2400 ; retract 0.8mm',
        'G0 Z0.6 F3000 ; Z-hop lift (+0.4mm travel)',
        'G0 X0 Y0 F9000 ; rapid travel across diagonal (sqrt(50^2 + 50^2) ≈ 70.710678mm)',
        'G0 Z0.2 F3000 ; Z-hop drop (+0.4mm travel)',
        'G1 E5.0 F2400 ; unretract 0.8mm',
        'G1 X0 Y50 E7.5 F1800 ; print 50mm segment (total extrude = 150mm)',
      ].join('\n')

      const audit = auditGcodePrintability(gcode)
      expect(audit.retractionCount).toBe(1)
      expect(audit.unretractionCount).toBe(1)
      expect(audit.zHopCount).toBe(1)
      expect(audit.maxFeedrateMmPerMin).toBe(9000)

      // Travel distance = Z-hop up (0.4) + XY travel (sqrt(5000)) + Z-hop down (0.4)
      const expectedTravel = 0.4 + Math.hypot(50, 50) + 0.4
      const expectedExtrude = 150
      expect(audit.travelToExtrudeRatio).toBeCloseTo(expectedTravel / expectedExtrude, 5)
    })

    it('supports M83 relative extrusion mode', () => {
      const gcode = [
        'G90',
        'M83',
        'G1 X10 Y0 E0.5 F1500',
        'G1 E-1.2 F2100',
        'G0 Z0.5 F3600',
        'G0 X20 Y0 F6000',
        'G0 Z0.2 F3600',
        'G1 E1.2 F2100',
        'G1 X30 Y0 E0.5 F1500',
      ].join('\n')

      const audit = auditGcodePrintability(gcode)
      expect(audit.retractionCount).toBe(1)
      expect(audit.unretractionCount).toBe(1)
      expect(audit.zHopCount).toBe(1)
      expect(audit.maxFeedrateMmPerMin).toBe(6000)
      expect(audit.travelToExtrudeRatio).toBeGreaterThan(0.5)
    })
  })

  describe('cadJoints evaluateMechanismMobility3d', () => {
    it('computes spatial Kutzbach mobility for serial manipulators, four-bar linkages, and rigid structures', () => {
      // 1 grounded base + 1 moving link connected by 1 revolute joint -> M = 6*(1) - 5 = 1 (determinate)
      const singlePendulum = evaluateMechanismMobility3d(2, 1, [{ type: 'revolute' }])
      expect(singlePendulum.movingBodyCount).toBe(1)
      expect(singlePendulum.totalFreeJointDof).toBe(1)
      expect(singlePendulum.totalConstrainedDof).toBe(5)
      expect(singlePendulum.mobility).toBe(1)
      expect(singlePendulum.classification).toBe('kinematically-determinate')

      // 6-DOF serial robot arm: 1 base + 6 moving links, 6 revolute joints -> M = 6*6 - 6*5 = 6 (underconstrained / multi-DOF)
      const sixAxisArm = evaluateMechanismMobility3d(7, 1, [
        { type: 'revolute' },
        { type: 'revolute' },
        { type: 'revolute' },
        { type: 'revolute' },
        { type: 'revolute' },
        { type: 'revolute' },
      ])
      expect(sixAxisArm.mobility).toBe(6)
      expect(sixAxisArm.classification).toBe('underconstrained-mechanism')

      // Spatial RSSR 4-bar mechanism: 4 bodies (1 grounded, 3 moving), 2 revolute (f=1) + 2 spherical ball (f=3)
      // Constrained DOFs = 2*5 + 2*3 = 16; 6*3 - 16 = 2 (including passive spin of coupler link)
      const rssr = evaluateMechanismMobility3d(4, 1, [
        { type: 'revolute' },
        { type: 'ball' },
        { type: 'ball' },
        { type: 'revolute' },
      ])
      expect(rssr.totalFreeJointDof).toBe(8)
      expect(rssr.totalConstrainedDof).toBe(16)
      expect(rssr.mobility).toBe(2)
      expect(rssr.classification).toBe('underconstrained-mechanism')

      // Rigidly welded assembly: 2 bodies, 1 grounded, 1 fixed joint -> M = 6*1 - 6 = 0
      const welded = evaluateMechanismMobility3d(2, 1, [{ type: 'fixed' }])
      expect(welded.mobility).toBe(0)
      expect(welded.classification).toBe('rigid-or-overconstrained')
    })
  })
})
