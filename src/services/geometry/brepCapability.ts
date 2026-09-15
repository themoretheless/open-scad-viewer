/** Honest B-rep capability maturity for WASM / MCP / SemanticProgram seams. */

export type BrepCapabilityMaturity =
  | 'Unavailable'
  | 'ResearchOnly'
  | 'NumericUncertified'
  | 'AnalyticComplete'
  | 'Qualified'

export interface BrepCapabilityDescriptor {
  readonly id: string
  readonly maturity: BrepCapabilityMaturity
  readonly permitsTopologyChange: boolean
  readonly notes: string
  readonly qualificationPlan?: string
}

/** Never advertise Available/Qualified for unfinished gates. */
export const BREP_CAPABILITY_MATRIX: readonly BrepCapabilityDescriptor[] = Object.freeze([
  Object.freeze({
    id: 'planar-csg/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Orthogonal / convex / bounded planar CSG',
    qualificationPlan: 'docs/qualification/brep-native-workbench-boolean-v1.json',
  }),
  Object.freeze({
    id: 'analytic-constructors/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes: 'Cylinder/sphere/torus/revolve constructors with identity matrix',
  }),
  Object.freeze({
    id: 'analytic-boolean/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes:
      'G5e cylinder imprint BooleanCertificate + listed analytic SS (plane/cyl/cone/sphere/torus contacts); UV classify+missed-branch; no mesh/Manifold fallback',
    qualificationPlan: 'docs/qualification/analytic-boolean-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'intersection-queries/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes:
      'Affine SS + analytic_ss listed contacts may be Complete; general NURBS remain Incomplete/NumericallyResolved',
  }),
  Object.freeze({
    id: 'nurbs-ss-transverse-bicubic/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes:
      'G6 R1 expand: Complete-empty + planar transverse Complete line; overlap/coincident refuse; no Boolean imprint',
    qualificationPlan: 'docs/qualification/nurbs-ss-g6-evidence-v1.json',
  }),
  Object.freeze({
    id: 'analytic-fillet/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: true,
    notes: 'AF-01 cylindrical fillet on axis-aligned cuboid vertical edges; AF-N1 curved refuse',
    qualificationPlan: 'docs/qualification/analytic-fillet-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'analytic-shell/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: true,
    notes: 'AS-01 planar cuboid shell via shell_planar + certificate',
    qualificationPlan: 'docs/qualification/analytic-shell-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'analytic-solid-loft/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: true,
    notes: 'ASL-01 SectionMatch ruled solid loft; surface-as-solid refuse',
    qualificationPlan: 'docs/qualification/analytic-solid-loft-1-evidence-v1.json',
  }),
  Object.freeze({
    id: 'step-interchange/1',
    maturity: 'AnalyticComplete' as const,
    permitsTopologyChange: false,
    notes:
      'Analytic ADVANCED_FACE / MANIFOLD_SOLID_BREP export+import; FACETED_BREP refuse; not STL/OBJ',
    qualificationPlan: 'docs/qualification/step-interchange-1-evidence-v1.json',
  }),
])

export function brepCapability(id: string): BrepCapabilityDescriptor | undefined {
  return BREP_CAPABILITY_MATRIX.find(entry => entry.id === id)
}

export function assertBrepCapabilityAllowsTopology(id: string): void {
  const entry = brepCapability(id)
  if (!entry) throw new Error(`Unknown B-rep capability: ${id}`)
  if (entry.maturity === 'Unavailable' || entry.maturity === 'ResearchOnly') {
    throw new Error(`B-rep capability ${id} is ${entry.maturity}; refuse topology change`)
  }
  if (!entry.permitsTopologyChange) {
    throw new Error(`B-rep capability ${id} does not permit topology change`)
  }
}

/** Rollback drill: revoked / stale LKG must not fall through to Manifold. */
export function assertNoManifoldFallbackOnBrepFailure(capabilityId: string): void {
  const entry = brepCapability(capabilityId)
  if (!entry) throw new Error(`Unknown B-rep capability: ${capabilityId}`)
  if (entry.maturity === 'Unavailable') {
    throw new Error(`B-rep capability ${capabilityId} unavailable; refuse Manifold cross-route`)
  }
}
