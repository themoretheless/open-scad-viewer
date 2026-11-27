import type { BrepCapabilityDescriptor } from './brepCapability'

export const NURBS_CAPABILITIES: readonly BrepCapabilityDescriptor[] = Object.freeze([
  Object.freeze({
    id: 'nurbs-foundation/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Positive denominators, span hulls, planar regularity, exact insertion/elevation.',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-1.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/2',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Bounded roots, surface projection, normal cones and certified simplification.',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-2.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/3',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Periodic editing, bounded simplification/projection, singularities and approximate fitting.',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-3.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/4',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Interval projection, singularities, nonlinear maps and bounded cloud fitting.',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-4.json',
  }),
  Object.freeze({
    id: 'nurbs-foundation/5',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'Bounded CC/CS intersections with multiplicity, trim maps and context evidence.',
    qualificationPlan: 'docs/qualification/plans/nurbs-foundation-5.json',
  }),
  Object.freeze({
    id: 'nurbs-cc-cs/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'CC/CS for positive-weight degrees 1..25; resource/conditioning limits refuse.',
    qualificationPlan: 'docs/qualification/plans/nurbs-cc-cs-1.json',
  }),
  Object.freeze({
    id: 'nurbs-ss/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: false,
    notes: 'SS for positive-weight degrees 1..25 with branch/trim evidence; Boolean separate.',
    qualificationPlan: 'docs/qualification/plans/nurbs-ss-1.json',
  }),
  Object.freeze({
    id: 'nurbs-boolean/1',
    maturity: 'Qualified' as const,
    permitsTopologyChange: true,
    notes: 'Graph × affine parallelotope Boolean; tangency, coincidence and freeform pairs refuse.',
    qualificationPlan: 'docs/qualification/plans/nurbs-boolean-1.json',
  }),
])
