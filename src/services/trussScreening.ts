import {callGeometryRust} from './geometry/kernel'
import type {TrussInput, TrussResponse} from './trussAnalysis'

export interface AxialLimits {
  tensionMpa: number
  compressionMpa: number
  safetyFactor: number
}

export interface AxialScreeningRow {
  memberIndex: number
  utilization: number
  governingResponse: number
  stressMpa: number
  status: 'overloaded' | 'low-demand' | 'within-axial-limit'
}

/** Transport only; validation, envelope and ranking are native. */
export function screenTrussMembers(model: TrussInput, responses: TrussResponse[], limits: AxialLimits): AxialScreeningRow[] {
  return callGeometryRust('truss_screen', {
    areasMm2: model.members.map(member => member.areaMm2),
    forcesN: responses.map(response => response.axialForcesN), limits,
  })
}

export interface PrintStrengthProfile {
  material: string
  grade: string
  propertySource: string
  nozzleMm: number
  lineWidthMm: number
  layerHeightMm: number
  nozzleTempC: number
  bedTempC: number
}
export interface ValidatedPrintProfile {
  profile: PrintStrengthProfile
  warnings: ('layer-above-80-percent-nozzle' | 'line-width-not-greater-than-layer')[]
  propertyModel: 'user-supplied-isotropic-axial'
}
export function validatePrintStrengthProfile(profile: PrintStrengthProfile): ValidatedPrintProfile {
  return callGeometryRust('print_strength_profile', {profile})
}
