export const LANGUAGE_CONTRACT_ID = 'openscad-viewer-subset' as const
export const LANGUAGE_CONTRACT_VERSION = 1 as const
export const LANGUAGE_SEMANTICS_REVISION = '1.0.0' as const
export const LANGUAGE_LIMITS_PROFILE = 'browser-v1' as const

export type LanguageDiagnosticCode =
  | 'E_FEATURE_INCLUDE'
  | 'E_FEATURE_USE'
  | 'E_FEATURE_USER_FUNCTION'
  | 'E_FEATURE_VIEWPORT_MODIFIER'

export const LANGUAGE_CONTRACT = Object.freeze({
  id: LANGUAGE_CONTRACT_ID,
  version: LANGUAGE_CONTRACT_VERSION,
  semanticsRevision: LANGUAGE_SEMANTICS_REVISION,
  limitsProfile: LANGUAGE_LIMITS_PROFILE,
  compatibility: 'independent-versioned-subset' as const,
  supported: Object.freeze([
    'variables', 'expressions', 'ranges', 'for', 'if', 'let', 'assert-statement',
    'user-modules', 'children', '2d-primitives', '3d-primitives', 'transforms',
    'boolean-csg', 'hull', 'linear-extrude', 'rotate-extrude', 'projection', 'offset',
  ]),
  unsupported: Object.freeze([
    'include', 'use', 'user-functions', 'expression-assert', 'surface', 'import',
    'text', 'minkowski', 'official-runtime-extensions',
  ]),
})

export type LanguageContract = typeof LANGUAGE_CONTRACT
