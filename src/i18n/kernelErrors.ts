/**
 * Single presentation layer for geometry-kernel and direct-modeling errors (roadmap C4).
 *
 * Every code in `KERNEL_ERROR_CODES` resolves to a spec: a reason message key, the kind
 * of input that failed (so the UI can point at a body, face, edge, sketch, parameter field
 * or file) and a suggested next step. Texts live in `APP_STRINGS` under `kernelError.*`
 * and `kernelStep.*`; unknown codes fall back to a generic message that still shows the code.
 */
import { translate, type Language } from './appStrings'
import { KERNEL_ERROR_CODES, type KernelErrorCode } from './kernelErrorCodes'

export { KERNEL_ERROR_CODES, type KernelErrorCode }

export type KernelErrorInput = 'body' | 'face' | 'edge' | 'sketch' | 'curve' | 'surface' | 'field' | 'file' | 'none'

export const KERNEL_ERROR_REASONS = [
  'invalidInput',
  'refused',
  'unproven',
  'resourceLimit',
  'topology',
  'importFile',
  'failed',
  'unavailable',
  'crash',
  'timeout',
  'transport',
  'cancelled',
  'busy',
  'extrudeHeight',
  'revolveAngle',
  'revolveAxis',
  'loftSections',
  'sketchSelfIntersection',
  'stepUnsupportedRoot',
  'shellRefused',
  'profileMissing',
  'dimensionPositive',
  'booleanOperands',
  'booleanDuplicate',
  'invalidSize',
  'invalidSelection',
  'stepAssemblyContent',
] as const
export type KernelErrorReason = (typeof KERNEL_ERROR_REASONS)[number]

export const KERNEL_ERROR_STEPS = [
  'checkInput',
  'changeGeometry',
  'changeParameters',
  'simplify',
  'undo',
  'checkFile',
  'retry',
  'reload',
  'wait',
  'enterNonzeroHeight',
  'enterAngle',
  'moveAxis',
  'selectSections',
  'redrawSketch',
  'exportStep',
  'useFacetedShell',
  'selectProfile',
  'enterPositive',
  'fillOperands',
  'removeDuplicate',
  'selectEdges',
  'exportBodiesOnly',
] as const
export type KernelErrorStep = (typeof KERNEL_ERROR_STEPS)[number]

export interface KernelErrorSpec {
  reason: KernelErrorReason
  input: KernelErrorInput
  step: KernelErrorStep
  /** Inspector field id (`data-field`) to mark with `aria-invalid`. */
  field?: string
}

/** Codes that need a message more specific than their family. */
const SPECIFIC: Partial<Record<KernelErrorCode, KernelErrorSpec>> = {
  DIRECT_EXTRUDE_HEIGHT: { reason: 'extrudeHeight', input: 'field', field: 'height', step: 'enterNonzeroHeight' },
  BREP_INVALID_ANGLE: { reason: 'revolveAngle', input: 'field', field: 'revolveAngle', step: 'enterAngle' },
  DIRECT_REVOLVE_AXIS: { reason: 'revolveAxis', input: 'field', field: 'revolveOffset', step: 'moveAxis' },
  DIRECT_LOFT_SECTIONS: { reason: 'loftSections', input: 'sketch', step: 'selectSections' },
  DIRECT_SKETCH_SELF_INTERSECTION: { reason: 'sketchSelfIntersection', input: 'sketch', step: 'redrawSketch' },
  STEP_UNSUPPORTED_ROOT: { reason: 'stepUnsupportedRoot', input: 'file', step: 'exportStep' },
  BREP_ANALYTIC_SHELL_REFUSED: { reason: 'shellRefused', input: 'body', step: 'useFacetedShell' },
  DIRECT_PROFILE_MISSING: { reason: 'profileMissing', input: 'sketch', step: 'selectProfile' },
  DIRECT_DIMENSION_POSITIVE: { reason: 'dimensionPositive', input: 'field', field: 'dimension', step: 'enterPositive' },
  DIRECT_BOOLEAN_OPERANDS: { reason: 'booleanOperands', input: 'body', step: 'fillOperands' },
  DIRECT_BOOLEAN_DUPLICATE: { reason: 'booleanDuplicate', input: 'body', step: 'removeDuplicate' },
  BREP_INVALID_SIZE: { reason: 'invalidSize', input: 'field', field: 'size', step: 'enterPositive' },
  BREP_INVALID_SELECTION: { reason: 'invalidSelection', input: 'edge', step: 'selectEdges' },
  DIRECT_STEP_ASSEMBLY_CONTENT: { reason: 'stepAssemblyContent', input: 'none', step: 'exportBodiesOnly' },
  CAD_CRASH: { reason: 'crash', input: 'none', step: 'retry' },
  CAD_TIMEOUT: { reason: 'timeout', input: 'none', step: 'simplify' },
  WORKER_TIMEOUT: { reason: 'timeout', input: 'none', step: 'simplify' },
  CAD_TRANSPORT: { reason: 'transport', input: 'none', step: 'retry' },
  CAD_PROTOCOL: { reason: 'transport', input: 'none', step: 'retry' },
  WORKER_RESULT_UNPUBLISHABLE: { reason: 'transport', input: 'none', step: 'retry' },
  CAD_CANCELLED: { reason: 'cancelled', input: 'none', step: 'retry' },
  CAD_BUSY: { reason: 'busy', input: 'none', step: 'wait' },
  CAD_DISPOSED: { reason: 'unavailable', input: 'none', step: 'reload' },
  CAD_STARTUP: { reason: 'unavailable', input: 'none', step: 'reload' },
  CAPABILITY_UNAVAILABLE: { reason: 'unavailable', input: 'none', step: 'reload' },
  ENGINE_UNAVAILABLE: { reason: 'unavailable', input: 'none', step: 'reload' },
  GEOMETRY_ABI_MISMATCH: { reason: 'unavailable', input: 'none', step: 'reload' },
  CAD_SOURCE_BODY_RESTORE: { reason: 'topology', input: 'body', step: 'undo' },
}

function familyInput(code: string): KernelErrorInput {
  if (/FILLET|CHAMFER|BLEND|BEVEL|EDGE|MITER|CORNER|TRIHEDRAL|VALENCE/.test(code)) return 'edge'
  if (/FACE|SURFACE|_SS_|UV|SKIN|PATCH|OFFSET/.test(code)) return 'face'
  if (/PROFILE|SKETCH|POLYGON|SECTION|SWEEP|GEAR|THREAD/.test(code)) return 'sketch'
  if (/^(NURBS|BEZIER|FRAME)_/.test(code)) return 'curve'
  if (/^(E_IMPORT|MESH_IMPORT|MESH_IO|STEP_)|_(STEP|IGES)_|EXCHANGE/.test(code)) return 'file'
  if (/^(BREP|SOLID|BONDED|MINKOWSKI|MESH|COMBO|E_DIRECT)_/.test(code)) return 'body'
  return 'none'
}

/** Family rules in priority order; the first match wins. */
function familySpec(code: string): KernelErrorSpec {
  const input = familyInput(code)
  if (/RESOURCE_LIMIT|BUDGET|_WORK(_LIMIT)?$|EXHAUSTED|TOO_MANY|_LIMIT$/.test(code))
    return { reason: 'resourceLimit', input, step: 'simplify' }
  if (input === 'file') return { reason: 'importFile', input, step: 'checkFile' }
  if (/UNPROVEN|UNPROVED|INDETERMINATE|PENDING|CONTRADICTION/.test(code))
    return { reason: 'unproven', input, step: 'changeParameters' }
  if (/INPUT|INVALID|_INDEX$|SELECTION|_NAME$/.test(code)) return { reason: 'invalidInput', input, step: 'checkInput' }
  if (/REFUSED|UNSUPPORTED|UNRESOLVED/.test(code)) return { reason: 'refused', input, step: 'changeGeometry' }
  if (/TOPOLOGY|RESTORE|SNAPSHOT|HISTORY|REFERENCE|SESSION/.test(code))
    return { reason: 'topology', input, step: 'undo' }
  return { reason: 'failed', input, step: 'changeParameters' }
}

const KNOWN = new Set<string>(KERNEL_ERROR_CODES)

/** Spec for a known code, or `undefined` when the code is not registered. */
export function kernelErrorSpec(code: string): KernelErrorSpec | undefined {
  if (!KNOWN.has(code)) return undefined
  return SPECIFIC[code as KernelErrorCode] ?? familySpec(code)
}

export interface KernelErrorContext {
  /** Display name of the failing scene object, prefixed to the message. */
  target?: string | undefined
  /** Scene ids to highlight with the `invalid` selection state. */
  targetIds?: readonly string[] | undefined
}

export interface KernelErrorPresentation {
  code?: string
  /** Full localized text: `target: reason step`. */
  message: string
  reason: string
  step: string
  input: KernelErrorInput
  field?: string
  targetIds: readonly string[]
}

export function errorCode(error: unknown): string | undefined {
  if (!error || typeof error !== 'object' || !('code' in error)) return undefined
  const code = (error as { code: unknown }).code
  // The CAD worker labels every uncoded throw `CAD_OPERATION`; its own text is the best message.
  return typeof code === 'string' && code && code !== 'CAD_OPERATION' ? code : undefined
}

function errorField(error: unknown): string | undefined {
  if (!error || typeof error !== 'object' || !('field' in error)) return undefined
  const field = (error as { field: unknown }).field
  return typeof field === 'string' ? field : undefined
}

function errorTargets(error: unknown): readonly string[] {
  if (!error || typeof error !== 'object' || !('targetIds' in error)) return []
  const ids = (error as { targetIds: unknown }).targetIds
  return Array.isArray(ids) ? ids.filter((id): id is string => typeof id === 'string') : []
}

/**
 * Localized description of any thrown value. Coded errors get the registry message and
 * a next step; uncoded errors keep their own (possibly already localized) text.
 */
export function presentKernelError(
  error: unknown,
  lang: Language,
  context: KernelErrorContext = {},
): KernelErrorPresentation {
  const code = errorCode(error)
  const prefix = context.target ? `${context.target}: ` : ''
  const ids = [...errorTargets(error), ...(context.targetIds ?? [])]
  const message =
    error && typeof error === 'object' && 'message' in error ? (error as { message: unknown }).message : undefined
  const raw = typeof message === 'string' ? message : String(error)
  if (!code) {
    return { message: prefix + raw, reason: raw, step: '', input: 'none', targetIds: ids }
  }
  const spec = kernelErrorSpec(code)
  const params = { code }
  const reason = translate(lang, `kernelError.${spec?.reason ?? 'unknown'}`, params)
  const step = translate(lang, `kernelStep.${spec?.step ?? 'changeParameters'}`, params)
  const field = errorField(error) ?? spec?.field
  // Family messages are generic; kernel texts are English, so only the English UI keeps them as detail.
  const detail =
    lang === 'en' && !(code in SPECIFIC) && raw.trim()
      ? ` ${translate(lang, 'kernelErrorDetail', { detail: raw.trim() })}`
      : ''
  return {
    code,
    message: `${prefix}${reason} ${step}${detail}`,
    reason,
    step,
    input: spec?.input ?? 'none',
    ...(field ? { field } : {}),
    targetIds: ids,
  }
}

export const LANGUAGE_ERROR_REASONS = [
  'unsupportedFeature',
  'projectRequired',
  'fileMissing',
  'invalidData',
  'unsupported',
  'limit',
  'nonManifold',
  'empty',
  'failed',
] as const
export type LanguageErrorReason = (typeof LANGUAGE_ERROR_REASONS)[number]

/**
 * Reason family for an OpenSCAD language diagnostic code (`LanguageDiagnosticCode`,
 * `E_FEATURE_*`, `E_SURFACE_*`, `E_IMPORT_*`, `E_TEXT_*`); `undefined` for other codes.
 */
export function languageErrorReason(code: string): LanguageErrorReason | undefined {
  if (/^E_FEATURE_/.test(code)) return 'unsupportedFeature'
  if (!/^E_(SURFACE|IMPORT|TEXT)_/.test(code)) return undefined
  if (/_PROJECT_(REQUIRED|MISMATCH)$|_FILE_REQUIRED$/.test(code)) return 'projectRequired'
  if (/_PATH_INVALID$|_FILE_MISSING$|_FONT_NOT_FOUND$/.test(code)) return 'fileMissing'
  if (/_LIMIT$/.test(code)) return 'limit'
  if (/_NON_MANIFOLD$/.test(code)) return 'nonManifold'
  if (/_EMPTY$/.test(code)) return 'empty'
  if (/UNSUPPORTED|UNAVAILABLE/.test(code)) return 'unsupported'
  if (/INVALID|ENCODING/.test(code)) return 'invalidData'
  return 'failed'
}

/**
 * Localized text for a failed OpenSCAD build. Language diagnostics get a reason and a next
 * step in the UI language plus the code; the evaluator's own text (line, column, the
 * offending name) stays as detail. Other errors keep their message unchanged.
 */
export function presentLanguageError(
  error: { code?: string | undefined; message: string },
  t: (key: string) => string,
): string {
  const reason = error.code ? languageErrorReason(error.code) : undefined
  if (!reason) return error.message
  const detail = error.message.trim()
  return `${t(`languageError.${reason}`)} (${error.code})${detail ? ` ${detail}` : ''}`
}

/** Typed refusal raised by TypeScript-side validation before the kernel is called. */
export class DirectInputError extends Error {
  constructor(
    readonly code: KernelErrorCode,
    message: string,
    readonly field?: string,
    readonly targetIds: readonly string[] = [],
  ) {
    super(message)
    this.name = 'DirectInputError'
  }
}
