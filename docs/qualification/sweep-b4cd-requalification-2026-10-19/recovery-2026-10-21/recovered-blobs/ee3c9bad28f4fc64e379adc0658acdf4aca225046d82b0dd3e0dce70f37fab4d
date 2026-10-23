export const LANGUAGE_CONTRACT_ID = 'openscad-viewer-subset' as const
export const LANGUAGE_CONTRACT_VERSION = 1 as const
export const LANGUAGE_SEMANTICS_REVISION = '1.0.0' as const
export const LANGUAGE_LIMITS_PROFILE = 'browser-v1' as const

export type LanguageDiagnosticCode =
  | 'E_FEATURE_INCLUDE'
  | 'E_FEATURE_USE'
  | 'E_FEATURE_USER_FUNCTION'
  | 'E_FEATURE_VIEWPORT_MODIFIER'
  | 'E_SURFACE_PROJECT_REQUIRED'
  | 'E_SURFACE_FILE_REQUIRED'
  | 'E_SURFACE_PATH_INVALID'
  | 'E_SURFACE_FILE_MISSING'
  | 'E_SURFACE_DAT_ENCODING'
  | 'E_SURFACE_DAT_INVALID'
  | 'E_SURFACE_PNG_INVALID'
  | 'E_SURFACE_PNG_UNSUPPORTED'
  | 'E_SURFACE_DIMENSION_LIMIT'
  | 'E_SURFACE_TRIANGLE_LIMIT'
  | 'E_SURFACE_NON_MANIFOLD'
  | 'E_IMPORT_PROJECT_REQUIRED'
  | 'E_IMPORT_FILE_REQUIRED'
  | 'E_IMPORT_PATH_INVALID'
  | 'E_IMPORT_FILE_MISSING'
  | 'E_IMPORT_FORMAT_UNSUPPORTED'
  | 'E_IMPORT_ENCODING'
  | 'E_IMPORT_INVALID_DATA'
  | 'E_IMPORT_UNSUPPORTED_FEATURE'
  | 'E_IMPORT_ARGUMENT_INVALID'
  | 'E_IMPORT_LIMIT'
  | 'E_IMPORT_EMPTY'
  | 'E_IMPORT_NEF3_UNAVAILABLE'
  | 'E_IMPORT_NON_MANIFOLD'
  | 'E_TEXT_PROJECT_REQUIRED'
  | 'E_TEXT_PROJECT_MISMATCH'
  | 'E_TEXT_PARAMETER_INVALID'
  | 'E_TEXT_PARAMETER_LIMIT'
  | 'E_TEXT_FONT_LIMIT'
  | 'E_TEXT_FONT_NOT_FOUND'
  | 'E_TEXT_FONT_INVALID'
  | 'E_TEXT_FONT_UNSUPPORTED'
  | 'E_TEXT_RUNTIME_UNAVAILABLE'
  | 'E_TEXT_GLYPH_LIMIT'
  | 'E_TEXT_PATH_LIMIT'
  | 'E_TEXT_VERTEX_LIMIT'
  | 'E_TEXT_SHAPING_FAILED'

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
