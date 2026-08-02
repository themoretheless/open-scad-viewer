import { createHash } from 'node:crypto'

/*
 * Independent parser for the hand-frozen Manifold qualification corpus.
 * Keep this module free of production imports and generated production schema:
 * changing an implementation tag, limit, or serializer must not update the
 * qualification oracle implicitly.
 */

const EXACT_CLAIMS = Object.freeze([
  'outcome/error literal and span',
  'warning order and text',
  'quality and reduced',
  'LME1 mesh bytes including identities/provenance',
  'LSE1 scene bytes including identities/inspection',
])

const SHA256 = /^[a-f0-9]{64}$/
const MAX_CASES = 100
const MAX_SOURCE_CODE_UNITS = 250_000
const MAX_TEXT_CODE_UNITS = 250_512

export type ReferenceFrozenMetricPolicy = Readonly<{
  absolute: number
  relative: number
}>

export type ReferenceFrozenLegacySuccess = Readonly<{
  tag: 'success'
  lme1Sha256: string
  lme1ByteLength: number
  lse1Sha256: string
  lse1ByteLength: number
  warnings: readonly string[]
  volume: number
  surfaceArea: number
  quality: 'preview' | 'full'
  reduced: boolean
  meshCount: number
  sceneAssetCount: number
  sceneEntityCount: number
}>

export type ReferenceFrozenLegacyFailure = Readonly<{
  tag: 'error' | 'cancelled'
  name: string
  message: string
  code: string | null
  line: number | null
  column: number | null
  start: number | null
  end: number | null
}>

export type ReferenceFrozenLegacyExpected =
  | ReferenceFrozenLegacySuccess
  | ReferenceFrozenLegacyFailure

export type ReferenceFrozenLegacyCase = Readonly<{
  name: string
  source: string
  quality: 'preview' | 'full'
  expected: ReferenceFrozenLegacyExpected
}>

export type ReferenceFrozenLegacyManifest = Readonly<{
  schema: 'manifold-plan-oracle'
  version: 1
  comparison: Readonly<{
    meshFrame: 'LME1'
    sceneFrame: 'LSE1'
    exact: readonly string[]
    metrics: Readonly<{
      volume: ReferenceFrozenMetricPolicy
      surfaceArea: ReferenceFrozenMetricPolicy
    }>
  }>
  corpusSha256: string
  cases: readonly ReferenceFrozenLegacyCase[]
}>

export class ReferenceFrozenLegacyError extends TypeError {
  constructor(readonly path: string, detail: string) {
    super(`${path}: ${detail}`)
    this.name = 'ReferenceFrozenLegacyError'
  }
}

function fail(path: string, detail: string): never {
  throw new ReferenceFrozenLegacyError(path, detail)
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (value === null || Array.isArray(value) || typeof value !== 'object') {
    fail(path, 'expected a plain record')
  }
  const prototype = Object.getPrototypeOf(value)
  if (prototype !== Object.prototype && prototype !== null) fail(path, 'expected a plain record')
  return value as Record<string, unknown>
}

function exact(value: unknown, keys: readonly string[], path: string): Record<string, unknown> {
  const candidate = record(value, path)
  const actual = Object.keys(candidate)
  if (actual.length !== keys.length || keys.some(key => !Object.hasOwn(candidate, key))) {
    fail(path, `expected exact keys ${keys.join(', ')}`)
  }
  return candidate
}

function array(value: unknown, path: string, maximum: number): unknown[] {
  if (!Array.isArray(value) || value.length > maximum) {
    fail(path, `expected an array of at most ${maximum} items`)
  }
  for (let index = 0; index < value.length; index++) {
    if (!Object.hasOwn(value, index)) fail(`${path}[${index}]`, 'sparse arrays are forbidden')
  }
  return value
}

function text(value: unknown, path: string, maximum = MAX_TEXT_CODE_UNITS): string {
  if (typeof value !== 'string' || value.length > maximum) {
    fail(path, `expected text of at most ${maximum} UTF-16 code units`)
  }
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) fail(path, 'text is not well formed')
    } else if (unit >= 0xdc00 && unit <= 0xdfff) fail(path, 'text is not well formed')
  }
  return value
}

function finite(value: unknown, path: string, nonnegative = false): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || (nonnegative && value < 0)) {
    fail(path, `expected a ${nonnegative ? 'nonnegative ' : ''}finite number`)
  }
  return Object.is(value, -0) ? 0 : value
}

function integer(value: unknown, path: string, minimum = 0): number {
  if (!Number.isSafeInteger(value) || (value as number) < minimum) {
    fail(path, `expected a safe integer >= ${minimum}`)
  }
  return value as number
}

function nullableInteger(value: unknown, path: string, minimum = 0): number | null {
  return value === null ? null : integer(value, path, minimum)
}

function sha256(value: unknown, path: string): string {
  const hash = text(value, path, 64)
  if (!SHA256.test(hash)) fail(path, 'expected a lowercase SHA-256 digest')
  return hash
}

function metricPolicy(value: unknown, path: string): ReferenceFrozenMetricPolicy {
  const policy = exact(value, ['absolute', 'relative'], path)
  const absolute = finite(policy.absolute, `${path}.absolute`, true)
  const relative = finite(policy.relative, `${path}.relative`, true)
  if (absolute !== 1e-9 || relative !== 1e-9) {
    fail(path, 'qualification metric policy must remain absolute=relative=1e-9')
  }
  return Object.freeze({ absolute, relative })
}

function frozenExpected(value: unknown, path: string): ReferenceFrozenLegacyExpected {
  const candidate = record(value, path)
  if (candidate.tag === 'success') {
    const success = exact(candidate, [
      'tag', 'lme1Sha256', 'lme1ByteLength', 'lse1Sha256', 'lse1ByteLength',
      'warnings', 'volume', 'surfaceArea', 'quality', 'reduced', 'meshCount',
      'sceneAssetCount', 'sceneEntityCount',
    ], path)
    if (success.quality !== 'preview' && success.quality !== 'full') {
      fail(`${path}.quality`, 'expected preview or full')
    }
    if (typeof success.reduced !== 'boolean') fail(`${path}.reduced`, 'expected a boolean')
    const warnings = array(success.warnings, `${path}.warnings`, 32)
      .map((warning, index) => text(warning, `${path}.warnings[${index}]`, 512))
    return Object.freeze({
      tag: 'success',
      lme1Sha256: sha256(success.lme1Sha256, `${path}.lme1Sha256`),
      lme1ByteLength: integer(success.lme1ByteLength, `${path}.lme1ByteLength`, 1),
      lse1Sha256: sha256(success.lse1Sha256, `${path}.lse1Sha256`),
      lse1ByteLength: integer(success.lse1ByteLength, `${path}.lse1ByteLength`, 1),
      warnings: Object.freeze(warnings),
      volume: finite(success.volume, `${path}.volume`, true),
      surfaceArea: finite(success.surfaceArea, `${path}.surfaceArea`, true),
      quality: success.quality,
      reduced: success.reduced,
      meshCount: integer(success.meshCount, `${path}.meshCount`),
      sceneAssetCount: integer(success.sceneAssetCount, `${path}.sceneAssetCount`),
      sceneEntityCount: integer(success.sceneEntityCount, `${path}.sceneEntityCount`),
    })
  }
  const failure = exact(candidate, [
    'tag', 'name', 'message', 'code', 'line', 'column', 'start', 'end',
  ], path)
  if (failure.tag !== 'error' && failure.tag !== 'cancelled') {
    fail(`${path}.tag`, 'expected success, error, or cancelled')
  }
  if (failure.code !== null && typeof failure.code !== 'string') {
    fail(`${path}.code`, 'expected null or text')
  }
  const line = nullableInteger(failure.line, `${path}.line`, 1)
  const column = nullableInteger(failure.column, `${path}.column`, 1)
  const start = nullableInteger(failure.start, `${path}.start`)
  const end = nullableInteger(failure.end, `${path}.end`)
  if ((start === null) !== (end === null) || (start !== null && end! < start)) {
    fail(path, 'start/end must be a complete forward span')
  }
  return Object.freeze({
    tag: failure.tag,
    name: text(failure.name, `${path}.name`, 128),
    message: text(failure.message, `${path}.message`),
    code: failure.code === null ? null : text(failure.code, `${path}.code`, 80),
    line,
    column,
    start,
    end,
  })
}

function stableJson(value: unknown): string {
  if (value === null || typeof value === 'boolean' || typeof value === 'string') {
    return JSON.stringify(value)
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) fail('$hash', 'non-finite canonical number')
    return JSON.stringify(Object.is(value, -0) ? 0 : value)
  }
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`
  if (typeof value === 'object') {
    const candidate = value as Record<string, unknown>
    const keys = Object.keys(candidate).sort()
    return `{${keys.map(key => `${JSON.stringify(key)}:${stableJson(candidate[key])}`).join(',')}}`
  }
  fail('$hash', `unsupported canonical value ${typeof value}`)
}

export function referenceFrozenLegacySha256(value: string | Uint8Array): string {
  return createHash('sha256').update(value).digest('hex')
}

export function referenceFrozenLegacyCorpusSha256(cases: readonly unknown[]): string {
  return referenceFrozenLegacySha256(stableJson(cases))
}

export function referenceValidateFrozenLegacyManifest(
  value: unknown,
): ReferenceFrozenLegacyManifest {
  const manifest = exact(value, [
    'schema', 'version', 'comparison', 'corpusSha256', 'cases',
  ], '$')
  if (manifest.schema !== 'manifold-plan-oracle') fail('$.schema', 'unknown manifest schema')
  if (manifest.version !== 1) fail('$.version', 'unsupported manifest version')
  const comparison = exact(manifest.comparison, [
    'meshFrame', 'sceneFrame', 'exact', 'metrics',
  ], '$.comparison')
  if (comparison.meshFrame !== 'LME1') fail('$.comparison.meshFrame', 'expected LME1')
  if (comparison.sceneFrame !== 'LSE1') fail('$.comparison.sceneFrame', 'expected LSE1')
  const exactClaims = array(comparison.exact, '$.comparison.exact', EXACT_CLAIMS.length)
    .map((claim, index) => text(claim, `$.comparison.exact[${index}]`, 128))
  if (exactClaims.length !== EXACT_CLAIMS.length
    || exactClaims.some((claim, index) => claim !== EXACT_CLAIMS[index])) {
    fail('$.comparison.exact', 'exact qualification claims changed')
  }
  const metrics = exact(comparison.metrics, ['volume', 'surfaceArea'], '$.comparison.metrics')
  const cases = array(manifest.cases, '$.cases', MAX_CASES).map((item, index) => {
    const path = `$.cases[${index}]`
    const entry = exact(item, ['name', 'source', 'quality', 'expected'], path)
    if (entry.quality !== 'preview' && entry.quality !== 'full') {
      fail(`${path}.quality`, 'expected preview or full')
    }
    const expected = frozenExpected(entry.expected, `${path}.expected`)
    if (expected.tag === 'success' && expected.quality !== entry.quality) {
      fail(`${path}.expected.quality`, 'success quality disagrees with its invocation')
    }
    return Object.freeze({
      name: text(entry.name, `${path}.name`, 160),
      source: text(entry.source, `${path}.source`, MAX_SOURCE_CODE_UNITS),
      quality: entry.quality,
      expected,
    })
  })
  if (cases.length === 0) fail('$.cases', 'qualification corpus is empty')
  const names = new Set(cases.map(item => item.name))
  if (names.size !== cases.length) fail('$.cases', 'case names must be unique')
  const corpusSha256 = sha256(manifest.corpusSha256, '$.corpusSha256')
  const actualCorpusHash = referenceFrozenLegacyCorpusSha256(manifest.cases as unknown[])
  if (actualCorpusHash !== corpusSha256) fail('$.corpusSha256', 'corpus digest mismatch')
  return Object.freeze({
    schema: 'manifold-plan-oracle',
    version: 1,
    comparison: Object.freeze({
      meshFrame: 'LME1',
      sceneFrame: 'LSE1',
      exact: Object.freeze(exactClaims),
      metrics: Object.freeze({
        volume: metricPolicy(metrics.volume, '$.comparison.metrics.volume'),
        surfaceArea: metricPolicy(metrics.surfaceArea, '$.comparison.metrics.surfaceArea'),
      }),
    }),
    corpusSha256,
    cases: Object.freeze(cases),
  })
}

export function referenceFrozenMetricEqual(
  expected: number,
  actual: number,
  policy: ReferenceFrozenMetricPolicy,
): boolean {
  if (!Number.isFinite(expected) || !Number.isFinite(actual)) return false
  const difference = Math.abs(expected - actual)
  return difference <= policy.absolute
    + policy.relative * Math.max(Math.abs(expected), Math.abs(actual))
}
