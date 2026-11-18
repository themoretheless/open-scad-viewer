import { callGeometryRust } from './geometry/kernel'
import type { NurbsSurface } from './nurbsSurface'

export interface IsolatedSsContactRequest {
  a: NurbsSurface
  b: NurbsSurface
}

/** Rational values are transported as canonical decimal integers or n/d strings. */
interface IsolatedSsContactPoint {
  uvA: [string, string]
  uvB: [string, string]
  centerA: [string, string]
  coversEntireSurfaces: true
}

export type IsolatedSsContactReport = IsolatedSsContactPoint &
  (
    | { proof: 'definite-hessian' }
    | {
        proof: 'definite-powers'
        powers: [number, number]
        diagonalCoefficients: [string, string]
        mixedCoefficient: string
      }
  )

function rational(value: unknown): value is string {
  return typeof value === 'string' && /^(?:0|-?[1-9][0-9]*(?:\/[1-9][0-9]*)?)(?![\s\S])/.test(value)
}

function rationalPair(values: unknown): values is [string, string] {
  return Array.isArray(values) && values.length === 2 && rational(values[0]) && rational(values[1])
}

export function validIsolatedSsContactReport(value: unknown): value is IsolatedSsContactReport {
  const report = value as Partial<IsolatedSsContactReport> | null
  if (
    report?.coversEntireSurfaces !== true ||
    !rationalPair(report.uvA) ||
    !rationalPair(report.uvB) ||
    !rationalPair(report.centerA)
  ) {
    return false
  }
  if (report.proof === 'definite-hessian') return true
  return (
    report.proof === 'definite-powers' &&
    rationalPair(report.diagonalCoefficients) &&
    rational(report.mixedCoefficient) &&
    Array.isArray(report.powers) &&
    report.powers.length === 2 &&
    [report.powers[0], report.powers[1]].every(power => power === 2 || power === 4 || power === 6 || power === 8)
  )
}

export function qualifyIsolatedSsContact({ a, b }: IsolatedSsContactRequest): IsolatedSsContactReport {
  return callGeometryRust('brep_qualify_isolated_ss_contact', { a, b })
}
