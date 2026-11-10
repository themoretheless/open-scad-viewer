import type { AssemblyFrameExport } from '../../assemblyFrameExport'
import {
  finite,
  isNumericSequence,
  vector,
  type MainSolidExpectation,
  type MainSolidResults,
} from '../../mainSolidProtocol'
import type { StructuralSections } from '../../structuralSections'
import { isBoundaryConnectivity, isMaterialAudit } from '../../structuralSectionsValidation'
import type { TrussResponse } from '../../trussAnalysis'
import { arrayOf, numericSequenceOf } from '../mainSolidExpectation'
import { isNonnegativeFinite } from './predicates'

export function validAnimationFramesResult(value: unknown): boolean {
  const v = value as AssemblyFrameExport
  return (
    v?.frames?.length > 0 &&
    v.frames.length <= 300 &&
    finite(v.fps) &&
    v.fps > 0 &&
    v.width === 960 &&
    v.height === 640 &&
    v.frames.every(f => finite(f.time) && typeof f.svg === 'string' && f.svg.startsWith('<svg'))
  )
}

export function validAnimationSampleResult(value: unknown): boolean {
  const v = value as MainSolidResults['animationSample']
  return (
    v?.document?.version === 1 &&
    Array.isArray(v.document.bodies) &&
    Array.isArray(v.document.sketches) &&
    finite(v.camera?.yaw) &&
    finite(v.camera?.pitch)
  )
}

export function validDrawingPdfResult(value: unknown): boolean {
  return value instanceof Uint8Array && value.length > 0 && value.length <= 32 * 1024 * 1024
}

export function validDrawingSheetsResult(value: unknown): boolean {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    value.length <= 40 &&
    value.every(
      s =>
        s &&
        typeof s.svg === 'string' &&
        s.svg.length < 32 * 1024 * 1024 &&
        finite(s.width) &&
        finite(s.height) &&
        typeof s.title === 'string',
    )
  )
}

export function validStructuralSectionsResult(
  job: Extract<MainSolidExpectation, { kind: 'structuralSections' }>,
  value: unknown,
): boolean {
  const v = value as StructuralSections
  const planeAxes = { x: [1, 2], y: [2, 0], z: [0, 1] }[job.axis]
  return (
    v.modelKind === 'finished-mesh-sections-v1' &&
    v.axis === job.axis &&
    arrayOf(v.planeAxes, 2, n => Number.isInteger(n) && Number(n) >= 0 && Number(n) < 3) &&
    v.planeAxes.every((axis, i) => axis === planeAxes[i]) &&
    (v.selfIntersections === 'not-checked' || v.selfIntersections === 'checked-at-tolerance') &&
    !!v.sourceMesh &&
    isNumericSequence(v.sourceMesh.positions) &&
    v.sourceMesh.positions.length <= 900000 &&
    v.sourceMesh.positions.length % 3 === 0 &&
    numericSequenceOf(v.sourceMesh.positions, v.sourceMesh.positions.length, finite) &&
    isNumericSequence(v.sourceMesh.indices) &&
    v.sourceMesh.indices.length === v.triangleCount * 3 &&
    numericSequenceOf(
      v.sourceMesh.indices,
      v.sourceMesh.indices.length,
      n =>
        Number.isInteger(n) &&
        Number(n) >= 0 &&
        Number(n) < v.sourceMesh.positions.length / 3,
    ) &&
    finite(v.volumeMm3) &&
    v.volumeMm3 > 0 &&
    Number.isInteger(v.triangleCount) &&
    v.triangleCount > 0 &&
    v.triangleCount <= 100000 &&
    isBoundaryConnectivity(
      v.connectivity,
      v.triangleCount,
      v.sourceMesh.positions.length / 3,
    ) &&
    isMaterialAudit(v) &&
    Array.isArray(v.sections) &&
    v.sections.length === job.stations.length &&
    v.sections.length > 0 &&
    v.sections.length <= 64 &&
    Array.from(v.sections).every(
      (s, i) =>
        s &&
        s.positionMm === job.stations[i] &&
        finite(s.positionMm) &&
        typeof s.material === 'boolean' &&
        Array.isArray(s.contours) &&
        (s.material
          ? !!s.properties &&
            finite(s.properties.areaMm2) &&
            s.properties.areaMm2 > 0 &&
            arrayOf(s.properties.centroidMm, 2, finite) &&
            finite(s.properties.iuuMm4) &&
            s.properties.iuuMm4 > 0 &&
            finite(s.properties.ivvMm4) &&
            s.properties.ivvMm4 > 0 &&
            finite(s.properties.iuvMm4)
          : s.properties === null),
    )
  )
}

export function validTrussResult(
  job: Extract<MainSolidExpectation, { kind: 'truss' }>,
  value: unknown,
): boolean {
  const v = value as TrussResponse
  const { nodes, members } = job
  return (
    nodes > 0 &&
    nodes <= 125 &&
    members > 0 &&
    members <= 400 &&
    arrayOf(v.displacementsMm, nodes, vector) &&
    arrayOf(v.reactionsN, nodes, vector) &&
    arrayOf(v.axialForcesN, members, finite) &&
    arrayOf(v.axialStressesMpa, members, finite) &&
    isNonnegativeFinite(v.maxDeflectionMm) &&
    isNonnegativeFinite(v.maxRelativeResidual) &&
    v.maxRelativeResidual <= 1e-9 &&
    Number.isInteger(v.freeDofs) &&
    v.freeDofs >= 0 &&
    v.freeDofs <= 3 * nodes
  )
}

export function validInspectResult(value: unknown): boolean {
  return (
    Array.isArray(value) &&
    value.every(
      v =>
        v &&
        typeof v.a === 'string' &&
        typeof v.b === 'string' &&
        finite(v.overlapMm3) &&
        finite(v.gapMm) &&
        (v.closestPoints === undefined ||
          v.closestPoints === null ||
          arrayOf(v.closestPoints, 2, vector)) &&
        (v.displayMeshOnly === undefined || v.displayMeshOnly === true),
    )
  )
}
