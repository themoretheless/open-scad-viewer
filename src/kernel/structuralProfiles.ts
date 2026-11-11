import {callGeometryRust} from './geometry/kernel'

/** Nominal dimensions of one rolled I-section, millimetres. */
export interface StructuralProfileDims {
  /** Overall section depth. */
  heightMm:number
  flangeWidthMm:number
  webThicknessMm:number
  /** Flange thickness; average for sloped-flange GOST 8239-89 beams. */
  flangeThicknessMm:number
}

export type StructuralProfileFamily =
  'ipe'|'heb'|'w'|'gost-8239'|'sto-aschm-b'|'sto-aschm-sh'|'sto-aschm-k'

export interface StructuralProfile extends StructuralProfileDims {
  designation:string
  family:StructuralProfileFamily
}

/** Geometric properties computed from the nominal dimensions by rectangular
 * decomposition — web plus two flange plates, NO fillet material. Tabulated
 * mill values sit a few percent above these (typically 3–7 %). GOST 8239-89
 * beams have sloped flanges: their weak-axis inertia is overestimated by up
 * to ~20 %. Engineering estimates, not a certified section table.
 */
export interface StructuralProfileProperties extends StructuralProfile {
  areaMm2:number
  /** Mass per metre at steel density 7850 kg/m³. */
  massKgM:number
  /** Strong axis is centroidal, parallel to the flanges. */
  iStrongMm4:number
  iWeakMm4:number
  wStrongMm3:number
  wWeakMm3:number
  /** Saint-Venant torsion constant, thin-walled strip sum. */
  jMm4:number
  /** Warping constant, doubly symmetric I approximation I_weak·(h−tf)²/4. */
  cwMm6:number
}

/** Looks up one rolled I-section by designation (IPE 200, HEB 300,
 * W250×32.7, I20 / И20, 20Б1, 20Ш1, 25К2 …). Case/space/×-insensitive,
 * Cyrillic Б/Ш/К/И accepted. Errors: PROFILE_UNKNOWN.
 */
export function lookupStructuralProfile(designation:string):StructuralProfileProperties {
  return callGeometryRust<StructuralProfileProperties>('structural_profiles',{designation})
}

/** Full catalogue for UI pickers, in family order. */
export function listStructuralProfiles():StructuralProfile[] {
  return callGeometryRust<{profiles:StructuralProfile[]}>('structural_profiles',{}).profiles
}
