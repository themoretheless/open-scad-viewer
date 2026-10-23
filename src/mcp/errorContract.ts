export const PUBLIC_ERROR_CODES = Object.freeze([
  'model_not_found',
  'revision_conflict',
  'source_syntax_error',
  'language_contract_unsupported',
  'engine_unavailable',
  'capability_unavailable',
  'artifact_too_large',
  'invalid_geometry',
  'invalid_argument',
  'quota_exceeded',
  'server_busy',
  'deadline_exceeded',
  'cancelled',
  'internal_error',
] as const)

export type PublicErrorCode = typeof PUBLIC_ERROR_CODES[number]

const FIXED_RETRY_POLICY: Readonly<Partial<Record<PublicErrorCode, boolean>>> = Object.freeze({
  model_not_found: false,
  revision_conflict: true,
  source_syntax_error: false,
  language_contract_unsupported: false,
  capability_unavailable: false,
  artifact_too_large: true,
  invalid_geometry: false,
  invalid_argument: false,
  quota_exceeded: false,
  server_busy: true,
  deadline_exceeded: true,
  cancelled: true,
  internal_error: true,
})

export function isPublicErrorCode(value: unknown): value is PublicErrorCode {
  return typeof value === 'string' && (PUBLIC_ERROR_CODES as readonly string[]).includes(value)
}

/** Null means the conditional contract lacks the details needed to attest it. */
export function expectedPublicErrorRetryable(
  code: PublicErrorCode,
  details?: Readonly<Record<string, unknown>>,
): boolean | null {
  if (code !== 'engine_unavailable') return FIXED_RETRY_POLICY[code] ?? null
  const cause = details?.availability_cause
  if (typeof cause !== 'string' || ![
    'not-deployed',
    'provider-missing',
    'readiness-timeout',
    'readiness-failed',
    'revoked',
    'quarantined',
  ].includes(cause)) return null
  return cause === 'readiness-timeout' || cause === 'readiness-failed'
}

export interface PublicErrorCodeDescription {
  code: string
  severity: 'recoverable' | 'fatal'
  summary: string
  remediationHint: string
}

const PUBLIC_ERROR_DESCRIPTIONS: Readonly<
  Record<PublicErrorCode, Omit<PublicErrorCodeDescription, 'code'>>
> = Object.freeze({
  model_not_found: {
    severity: 'recoverable',
    summary: 'The requested model ID or revision does not exist in the catalog.',
    remediationHint:
      'List saved models via the catalog tool or pass inline OpenSCAD source directly.',
  },
  revision_conflict: {
    severity: 'recoverable',
    summary: 'Optimistic concurrency check failed because the model revision advanced.',
    remediationHint:
      'Fetch the latest model state, merge your edits, and retry with the updated expected_revision.',
  },
  source_syntax_error: {
    severity: 'recoverable',
    summary: 'The OpenSCAD or project source failed lexical, syntactic, or asset validation.',
    remediationHint:
      'Inspect the reported line/column and diagnostic details, fix the syntax or missing asset, and re-run check.',
  },
  language_contract_unsupported: {
    severity: 'recoverable',
    summary: 'The leading @language contract header is unsupported or malformed.',
    remediationHint:
      'Use a supported @language contract header or omit unsupported engine directives.',
  },
  engine_unavailable: {
    severity: 'fatal',
    summary: 'The required geometry evaluation engine is unavailable or quarantined.',
    remediationHint:
      'Inspect openscad://capabilities or openscad://official-runtime; restart the server if the worker is quarantined.',
  },
  capability_unavailable: {
    severity: 'recoverable',
    summary: 'The source requests a capability not supported by the active geometry engine.',
    remediationHint:
      'Check the engine capability manifest and remove unsupported @requires declarations or features.',
  },
  artifact_too_large: {
    severity: 'recoverable',
    summary: 'The generated mesh or export artifact exceeded the maximum byte budget.',
    remediationHint:
      'Reduce mesh tessellation density ($fn/$fs/$fa), simplify geometry, or choose a compact binary format.',
  },
  invalid_geometry: {
    severity: 'recoverable',
    summary: 'Evaluation produced non-finite coordinates, degenerate transforms, or invalid topology.',
    remediationHint:
      'Ensure all dimensions, scale factors, and boolean operands are finite and non-degenerate.',
  },
  invalid_argument: {
    severity: 'recoverable',
    summary: 'One or more tool arguments or Customizer parameters failed validation.',
    remediationHint:
      'Verify parameter types, bounds, and allowed enum values against the tool schema before retrying.',
  },
  quota_exceeded: {
    severity: 'recoverable',
    summary: 'The model catalog storage quota has been reached.',
    remediationHint:
      'Check openscad_catalog_stats and delete or archive obsolete models before saving new revisions.',
  },
  server_busy: {
    severity: 'recoverable',
    summary: 'The geometry worker pool is currently at maximum concurrency.',
    remediationHint:
      'Back off for retry_after_ms and retry the request once active evaluations finish.',
  },
  deadline_exceeded: {
    severity: 'recoverable',
    summary: 'Geometry evaluation exceeded the configured execution timeout.',
    remediationHint:
      'Simplify expensive Minkowski/hull/boolean operations or lower curve subdivision ($fn) and retry.',
  },
  cancelled: {
    severity: 'recoverable',
    summary: 'The operation was cancelled before completion.',
    remediationHint:
      'Re-issue the tool call if the geometry result is still needed.',
  },
  internal_error: {
    severity: 'fatal',
    summary: 'An unexpected internal fault occurred in the MCP server or native runtime.',
    remediationHint:
      'Retry once; if the error persists, inspect the server logs using the correlation_id.',
  },
})

/**
 * Returns structured diagnostic metadata (`severity`, `summary`, and `remediationHint`)
 * for a public MCP error code to guide automated agent recovery.
 */
export function describePublicErrorCode(code: string): PublicErrorCodeDescription {
  const normalized = typeof code === 'string' ? code.trim() : ''
  if (isPublicErrorCode(normalized)) {
    const entry = PUBLIC_ERROR_DESCRIPTIONS[normalized]
    return {
      code: normalized,
      severity: entry.severity,
      summary: entry.summary,
      remediationHint: entry.remediationHint,
    }
  }
  return {
    code: normalized || 'unknown_error',
    severity: 'fatal',
    summary: `Unrecognized error code '${normalized || 'unknown_error'}'.`,
    remediationHint:
      'Inspect the full error payload and server logs before retrying with validated parameters.',
  }
}
