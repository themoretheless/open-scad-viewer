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
