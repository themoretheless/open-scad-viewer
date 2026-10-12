import { SEMANTIC_PROGRAM_CAPABILITY_GRAPH, SEMANTIC_PROGRAM_DIAGNOSTIC_CODES, SEMANTIC_PROGRAM_EXECUTION_FEATURE, SEMANTIC_PROGRAM_IDENTITY, SEMANTIC_PROGRAM_LIMITS, deriveSemanticCapabilityClosure } from '../../core/semanticProgram'
import type { SemanticDiagnosticTemplate, SemanticProgramCoreV1, SemanticProgramDiagnosticCode, SemanticProgramEnvelopeV1, SemanticProgramV1, SemanticSourceDescriptor, SemanticSourceSpan, SemanticStaticOperation } from '../../core/semanticProgram'
import type { SnapshotBudget } from '../semanticProgramValidator'
import { arrayValue, assertSchemaVersion, exactKeys, fail, finiteNumber, integer, isRecord, nullablePositive, stringValue, validateIdentityValue, validateOperations, validateSortedStrings, wellFormedUnicode } from './fail'
import { deepFreeze } from '../../core/objects'
import { isSemanticProgramCodecOwnedGraph } from '../semanticProgramCodec'
import { parseGeometrySourceRoutingHeader } from '../../core/geometryRouting'
import { sha256Hex } from '../../core/sha256'
import { validateExecution, validateResult } from './validateResult'
import { validateNodes } from './validateNodes'
import { validateOccurrenceProduction } from './validateOccurrenceProduction'
import { validateOccurrences } from './isChildrenExpansionContinuation'

export function validateDiagnosticTemplates(
  value: unknown,
  operations: readonly SemanticStaticOperation[],
): readonly SemanticDiagnosticTemplate[] {
  const templates = arrayValue(value, '$.core.diagnosticTemplates', SEMANTIC_PROGRAM_LIMITS.diagnostics)
  templates.forEach((template, index) => {
    const path = `$.core.diagnosticTemplates[${index}]`
    exactKeys(template, ['id', 'code', 'severity', 'operation', 'arguments'], path)
    if (template.id !== index) fail('E_SEMANTIC_ORDER', `${path}.id`, 'diagnostic template IDs must equal array positions')
    const code = stringValue(template.code, `${path}.code`, 64)
    if (!/^[A-Z][A-Z0-9_]{0,63}$/.test(code)) fail('E_SEMANTIC_FIELD', `${path}.code`, 'invalid diagnostic code')
    if (template.severity !== 'info' && template.severity !== 'warning' && template.severity !== 'error') {
      fail('E_SEMANTIC_FIELD', `${path}.severity`, 'unknown diagnostic severity')
    }
    if (template.severity === 'error' && code !== 'LEGACY_LANGUAGE_ERROR') {
      fail('E_SEMANTIC_FIELD', `${path}.code`, 'v1.2 admits only the frozen legacy terminal error template')
    }
    if (template.operation !== null) {
      integer(template.operation, `${path}.operation`, 0, Math.max(0, operations.length - 1))
      if (operations.length === 0) fail('E_SEMANTIC_REFERENCE', `${path}.operation`, 'diagnostic references absent operation')
    }
    const args = arrayValue(template.arguments, `${path}.arguments`, 128)
    const names = new Set<string>()
    args.forEach((argument, argumentIndex) => {
      const argumentPath = `${path}.arguments[${argumentIndex}]`
      exactKeys(argument, ['name', 'value'], argumentPath)
      const name = stringValue(argument.name, `${argumentPath}.name`, 128)
      if (names.has(name)) fail('E_SEMANTIC_ORDER', `${argumentPath}.name`, 'diagnostic argument names must be unique in authored order')
      names.add(name)
      validateIdentityValue(argument.value, `${argumentPath}.value`)
    })
  })
  return templates as unknown as readonly SemanticDiagnosticTemplate[]
}

export function validateCore(value: unknown): asserts value is SemanticProgramCoreV1 {
  exactKeys(value, [
    'schema', 'schemaVersion', 'requiredFeatures', 'identityVersion', 'language',
    'units', 'operations', 'occurrences', 'nodes', 'execution', 'result', 'declaredCapabilities',
    'capabilityClosure', 'diagnosticTemplates',
  ], '$.core')
  if (value.schema !== 'semantic-program-core') fail('E_SEMANTIC_SCHEMA', '$.core.schema', 'expected semantic-program-core')
  assertSchemaVersion(value.schemaVersion, '$.core.schemaVersion')
  const features = validateSortedStrings(value.requiredFeatures, '$.core.requiredFeatures', 64, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  if (features.length !== 1 || features[0] !== SEMANTIC_PROGRAM_EXECUTION_FEATURE) {
    fail('E_SEMANTIC_VERSION', '$.core.requiredFeatures', 'v1.2 requires exactly semantic.execution-v2')
  }
  if (value.identityVersion !== SEMANTIC_PROGRAM_IDENTITY) fail('E_SEMANTIC_VERSION', '$.core.identityVersion', 'unknown core identity version')
  exactKeys(value.language, ['contract', 'semanticsRevision', 'capabilityGraphVersion'], '$.core.language')
  if (value.language.contract !== 'legacy/current' && value.language.contract !== 'openscad-viewer/brep-1') fail('E_SEMANTIC_FIELD', '$.core.language.contract', 'unknown language contract')
  const expectedRevision = value.language.contract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0'
  if (value.language.semanticsRevision !== expectedRevision) fail('E_SEMANTIC_VERSION', '$.core.language.semanticsRevision', `expected ${expectedRevision}`)
  if (value.language.capabilityGraphVersion !== SEMANTIC_PROGRAM_CAPABILITY_GRAPH) fail('E_SEMANTIC_VERSION', '$.core.language.capabilityGraphVersion', 'unknown capability graph')
  exactKeys(value.units, ['length', 'angle', 'handedness', 'upAxis', 'matrixLayout', 'composition'], '$.core.units')
  if (value.units.length !== 'millimeter' || value.units.angle !== 'degree'
    || value.units.handedness !== 'right' || value.units.upAxis !== 'z'
    || value.units.matrixLayout !== 'column-major' || value.units.composition !== 'parent-times-local') {
    fail('E_SEMANTIC_TYPE', '$.core.units', 'v1 units/frame/matrix convention is fixed')
  }
  const operations = validateOperations(value.operations)
  const nodes = validateNodes(value.nodes, value.language.contract)
  const occurrences = validateOccurrences(value.occurrences, operations, nodes)
  const templates = validateDiagnosticTemplates(value.diagnosticTemplates, operations)
  const outputRoots = validateResult(value.result, nodes, occurrences)
  const execution = validateExecution(
    value.execution,
    nodes,
    occurrences,
    operations,
    outputRoots,
    templates,
    value.language.contract,
  )
  validateOccurrenceProduction(
    operations,
    occurrences,
    nodes,
    value.result as SemanticProgramCoreV1['result'],
    execution.discardedEffects,
    execution.terminal,
  )
  const declared = validateSortedStrings(value.declaredCapabilities, '$.core.declaredCapabilities', SEMANTIC_PROGRAM_LIMITS.declaredCapabilities, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  const closure = validateSortedStrings(value.capabilityClosure, '$.core.capabilityClosure', SEMANTIC_PROGRAM_LIMITS.capabilityClosure, /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/)
  const expectedClosure = deriveSemanticCapabilityClosure({
    language: value.language as unknown as SemanticProgramCoreV1['language'],
    nodes,
    result: value.result as SemanticProgramCoreV1['result'],
    operations,
    occurrences,
    declaredCapabilities: declared,
    diagnosticTemplates: templates,
  })
  if (closure.length !== expectedClosure.length || closure.some((capability, index) => capability !== expectedClosure[index])) {
    fail('E_SEMANTIC_CAPABILITY_CLOSURE', '$.core.capabilityClosure', 'closure does not exactly match authored and inferred capabilities')
  }
}

export function validateSpan(value: unknown, path: string, sourceLength: number, emptyAllowed: boolean): asserts value is SemanticSourceSpan {
  exactKeys(value, ['start', 'end'], path)
  const start = integer(value.start, `${path}.start`, 0, sourceLength)
  const end = integer(value.end, `${path}.end`, 0, sourceLength)
  if (end < start || (!emptyAllowed && end === start)) fail('E_SEMANTIC_PROVENANCE', path, 'invalid half-open UTF-16 source span')
}

export function validateSemanticProgramV1(value: unknown): asserts value is SemanticProgramEnvelopeV1 {
  exactKeys(value, ['schema', 'schemaVersion', 'source', 'core', 'provenance', 'tessellationIntents', 'diagnostics'], '$')
  if (value.schema !== 'semantic-program-envelope') fail('E_SEMANTIC_SCHEMA', '$.schema', 'expected semantic-program-envelope')
  assertSchemaVersion(value.schemaVersion, '$.schemaVersion')
  exactKeys(value.source, ['sha256', 'utf8ByteLength', 'utf16CodeUnitLength'], '$.source')
  if (typeof value.source.sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(value.source.sha256)) fail('E_SEMANTIC_FIELD', '$.source.sha256', 'expected lowercase SHA-256')
  integer(value.source.utf8ByteLength, '$.source.utf8ByteLength', 0, 4_000_000)
  const sourceLength = integer(value.source.utf16CodeUnitLength, '$.source.utf16CodeUnitLength', 0, 250_000)
  const core = value.core
  validateCore(core)

  const provenance = arrayValue(value.provenance, '$.provenance', SEMANTIC_PROGRAM_LIMITS.operations)
  if (provenance.length !== core.operations.length) fail('E_SEMANTIC_PROVENANCE', '$.provenance', 'every static operation needs exactly one source record')
  provenance.forEach((record, index) => {
    const path = `$.provenance[${index}]`
    exactKeys(record, ['operation', 'span', 'label'], path)
    if (record.operation !== index) fail('E_SEMANTIC_ORDER', `${path}.operation`, 'provenance must follow operation order')
    validateSpan(record.span, `${path}.span`, sourceLength, false)
    stringValue(record.label, `${path}.label`, 512)
  })

  const intents = arrayValue(value.tessellationIntents, '$.tessellationIntents', SEMANTIC_PROGRAM_LIMITS.tessellationIntents)
  if (core.language.contract === 'legacy/current' && intents.length !== 0) {
    fail('E_SEMANTIC_TYPE', '$.tessellationIntents', 'legacy/current materializes tessellation in SPC1 and cannot carry TSP1 intents')
  }
  let priorOccurrence = -1
  intents.forEach((intent, index) => {
    const path = `$.tessellationIntents[${index}]`
    exactKeys(intent, ['occurrence', 'chordTolerance', 'angularToleranceDegrees', 'minSegments', 'maxSegments'], path)
    const occurrence = integer(intent.occurrence, `${path}.occurrence`, 0, Math.max(0, core.occurrences.length - 1))
    if (core.occurrences.length === 0 || core.occurrences[occurrence].node === null) fail('E_SEMANTIC_REFERENCE', `${path}.occurrence`, 'tessellation intent needs a producing occurrence')
    if (occurrence <= priorOccurrence) fail('E_SEMANTIC_ORDER', `${path}.occurrence`, 'intents must be unique and ordered by occurrence')
    priorOccurrence = occurrence
    nullablePositive(intent.chordTolerance, `${path}.chordTolerance`)
    nullablePositive(intent.angularToleranceDegrees, `${path}.angularToleranceDegrees`)
    const minimum = intent.minSegments === null ? null : integer(intent.minSegments, `${path}.minSegments`, 3, 1_000_000)
    const maximum = intent.maxSegments === null ? null : integer(intent.maxSegments, `${path}.maxSegments`, 3, 1_000_000)
    if (minimum !== null && maximum !== null && maximum < minimum) fail('E_SEMANTIC_NUMBER', path, 'maxSegments cannot be below minSegments')
  })

  const diagnostics = arrayValue(value.diagnostics, '$.diagnostics', SEMANTIC_PROGRAM_LIMITS.diagnostics)
  if (diagnostics.length !== core.diagnosticTemplates.length) fail('E_SEMANTIC_PROVENANCE', '$.diagnostics', 'every diagnostic template needs exactly one source-bound presentation')
  diagnostics.forEach((diagnostic, index) => {
    const path = `$.diagnostics[${index}]`
    exactKeys(diagnostic, ['template', 'message', 'span'], path)
    if (diagnostic.template !== index) fail('E_SEMANTIC_ORDER', `${path}.template`, 'diagnostics must follow template order')
    stringValue(diagnostic.message, `${path}.message`, SEMANTIC_PROGRAM_LIMITS.diagnosticMessageCodeUnits)
    if (diagnostic.span !== null) validateSpan(diagnostic.span, `${path}.span`, sourceLength, true)
  })
}

export function snapshotJsonValue(value: unknown, path: string, depth: number, ancestors: Set<object>, budget: SnapshotBudget): unknown {
  if (++budget.values > SEMANTIC_PROGRAM_LIMITS.snapshotValues) fail('E_SEMANTIC_LIMIT', path, 'snapshot value budget exceeded')
  if (depth > SEMANTIC_PROGRAM_LIMITS.snapshotDepth) fail('E_SEMANTIC_LIMIT', path, 'snapshot depth exceeded')
  if (value === null || typeof value === 'boolean') return value
  if (typeof value === 'number') return finiteNumber(value, path)
  if (typeof value === 'string') return stringValue(value, path, SEMANTIC_PROGRAM_LIMITS.stringCodeUnits)
  if (typeof value !== 'object') fail('E_SEMANTIC_SCHEMA', path, 'only JSON data is allowed')
  if (Object.getOwnPropertySymbols(value).length !== 0) fail('E_SEMANTIC_FIELD', path, 'symbol fields are forbidden')
  if (ancestors.has(value)) fail('E_SEMANTIC_DAG', path, 'wire object graph cannot be cyclic')
  ancestors.add(value)
  try {
    if (Array.isArray(value)) {
      const keys = Reflect.ownKeys(value)
      if (keys.length !== value.length + 1 || keys[keys.length - 1] !== 'length'
        || keys.slice(0, -1).some((key, index) => key !== String(index))) {
        fail('E_SEMANTIC_FIELD', path, 'arrays must be dense without symbol, hidden, or extra fields')
      }
      return Array.from({ length: value.length }, (_, index) => {
        const descriptor = Object.getOwnPropertyDescriptor(value, String(index))
        if (!descriptor || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) fail('E_SEMANTIC_FIELD', `${path}[${index}]`, 'array entries must be enumerable data properties')
        return snapshotJsonValue(descriptor.value, `${path}[${index}]`, depth + 1, ancestors, budget)
      })
    }
    const prototype = Object.getPrototypeOf(value)
    if (prototype !== Object.prototype && prototype !== null) fail('E_SEMANTIC_SCHEMA', path, 'only plain JSON objects are allowed')
    const output: Record<string, unknown> = Object.create(null)
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key !== 'string') fail('E_SEMANTIC_FIELD', path, 'symbol fields are forbidden')
      if (!wellFormedUnicode(key)) fail('E_SEMANTIC_FIELD', path, 'object keys must be well-formed Unicode')
      const descriptor = Object.getOwnPropertyDescriptor(value, key)
      if (!descriptor || !Object.hasOwn(descriptor, 'value') || !descriptor.enumerable) fail('E_SEMANTIC_FIELD', `${path}.${key}`, 'object fields must be enumerable data properties')
      Object.defineProperty(output, key, {
        value: snapshotJsonValue(descriptor.value, `${path}.${key}`, depth + 1, ancestors, budget),
        enumerable: true,
        configurable: true,
        writable: true,
      })
    }
    return output
  } finally {
    ancestors.delete(value)
  }
}

export function validateCanonicalStringBudget(value: unknown): void {
  const encoder = new TextEncoder()
  const pending: Array<{ value: unknown; path: string }> = [{ value, path: '$' }]
  let bytes = 0
  const add = (text: string, path: string) => {
    bytes += encoder.encode(text).length
    if (bytes > SEMANTIC_PROGRAM_LIMITS.canonicalStringBytes) {
      fail('E_SEMANTIC_LIMIT', path, `canonical string/key bytes exceed ${SEMANTIC_PROGRAM_LIMITS.canonicalStringBytes}`)
    }
  }
  while (pending.length > 0) {
    const current = pending.pop()!
    if (typeof current.value === 'string') {
      add(current.value, current.path)
    } else if (Array.isArray(current.value)) {
      for (let index = current.value.length - 1; index >= 0; index--) pending.push({ value: current.value[index], path: `${current.path}[${index}]` })
    } else if (isRecord(current.value)) {
      const entries = Object.entries(current.value)
      for (let index = entries.length - 1; index >= 0; index--) {
        const [key, child] = entries[index]
        add(key, `${current.path}.<key>`)
        pending.push({ value: child, path: `${current.path}.${key}` })
      }
    }
  }
}

export function normalizeSemanticProgram(input: unknown, exactSource?: string): SemanticProgramV1 {
  let snapshot = snapshotJsonValue(input, '$', 0, new Set(), { values: 0 })
  if (!isRecord(snapshot)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program envelope')
  if (snapshot.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(snapshot)
  validateSemanticProgramV1(snapshot)
  const program = deepFreeze(snapshot)
  if (exactSource !== undefined) attestSemanticProgramSource(program, exactSource)
  return program
}

/**
 * Validates and freezes the graph produced exclusively by the bounded binary
 * decoder. Unlike the public normalizer, this does not duplicate that already
 * isolated, null-prototype JSON graph. Callers with arbitrary objects must use
 * normalizeSemanticProgram so descriptors and prototypes are snapshotted.
 */
export function normalizeDecodedSemanticProgram(input: unknown): SemanticProgramV1 {
  if (!isSemanticProgramCodecOwnedGraph(input)) {
    fail('E_SEMANTIC_SCHEMA', '$', 'in-place normalization requires a codec-owned decoded graph')
  }
  if (!isRecord(input)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program envelope')
  // Old wire prototypes cannot synthesize v1.2 execution evidence.
  if (input.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(input)
  validateSemanticProgramV1(input)
  return deepFreeze(input as unknown as SemanticProgramV1)
}

export function normalizeSemanticProgramCore(input: unknown): SemanticProgramCoreV1 {
  const snapshot = snapshotJsonValue(input, '$.core', 0, new Set(), { values: 0 })
  validateCanonicalStringBudget(snapshot)
  validateCore(snapshot)
  return deepFreeze(snapshot as unknown as SemanticProgramCoreV1)
}

/** In-place counterpart for the graph freshly allocated by ByteReader only. */
export function normalizeDecodedSemanticProgramCore(input: unknown): SemanticProgramCoreV1 {
  if (!isSemanticProgramCodecOwnedGraph(input)) {
    fail('E_SEMANTIC_SCHEMA', '$.core', 'in-place normalization requires a codec-owned decoded graph')
  }
  validateCanonicalStringBudget(input)
  validateCore(input)
  return deepFreeze(input as unknown as SemanticProgramCoreV1)
}

export function normalizeSemanticProgramCoreInput(input: unknown): SemanticProgramCoreV1 {
  let snapshot = snapshotJsonValue(input, '$', 0, new Set(), { values: 0 })
  if (!isRecord(snapshot)) fail('E_SEMANTIC_SCHEMA', '$', 'expected a semantic program core or envelope')
  if (snapshot.version === 0) {
    fail('E_SEMANTIC_RELOWER_REQUIRED', '$.version', 'flat semantic prototype requires exact-source re-lowering to schema 1.2')
  }
  validateCanonicalStringBudget(snapshot)
  if (snapshot.schema === 'semantic-program-core') {
    validateCore(snapshot)
    return deepFreeze(snapshot as unknown as SemanticProgramCoreV1)
  }
  validateSemanticProgramV1(snapshot)
  return deepFreeze(snapshot as unknown as SemanticProgramV1).core
}

export function semanticSourceDescriptor(source: string): SemanticSourceDescriptor {
  if (!wellFormedUnicode(source)) fail('E_SEMANTIC_BYTES', '$source', 'semantic source must be well-formed Unicode before UTF-8 encoding')
  if (source.length > SEMANTIC_PROGRAM_LIMITS.sourceCodeUnits) {
    fail('E_SEMANTIC_LIMIT', '$source', `semantic source exceeds ${SEMANTIC_PROGRAM_LIMITS.sourceCodeUnits} UTF-16 code units`)
  }
  const bytes = new TextEncoder().encode(source)
  if (bytes.length > 4_000_000) fail('E_SEMANTIC_LIMIT', '$source', 'semantic source exceeds 4000000 UTF-8 bytes')
  return Object.freeze({
    sha256: sha256Hex(bytes),
    utf8ByteLength: bytes.length,
    utf16CodeUnitLength: source.length,
  })
}

export function attestSemanticProgramSource(program: SemanticProgramV1, source: string): void {
  const actual = semanticSourceDescriptor(source)
  if (actual.sha256 !== program.source.sha256
    || actual.utf8ByteLength !== program.source.utf8ByteLength
    || actual.utf16CodeUnitLength !== program.source.utf16CodeUnitLength) {
    fail('E_SEMANTIC_PROVENANCE', '$.source', 'source bytes do not match the envelope attestation')
  }
  let route: ReturnType<typeof parseGeometrySourceRoutingHeader>
  try {
    route = parseGeometrySourceRoutingHeader(source)
  } catch (error) {
    fail('E_SEMANTIC_PROVENANCE', '$source', `source routing header is invalid: ${error instanceof Error ? error.message : String(error)}`)
  }
  if (route.languageContract !== program.core.language.contract) {
    fail('E_SEMANTIC_PROVENANCE', '$.core.language.contract', 'source routing language does not match the semantic core')
  }
  if (route.requiredCapabilities.length !== program.core.declaredCapabilities.length
    || route.requiredCapabilities.some((capability, index) => capability !== program.core.declaredCapabilities[index])) {
    fail('E_SEMANTIC_PROVENANCE', '$.core.declaredCapabilities', 'source routing requirements do not match authored semantic capabilities')
  }
  const assertScalarBoundary = (offset: number, path: string) => {
    if (offset > 0 && offset < source.length) {
      const prior = source.charCodeAt(offset - 1)
      const next = source.charCodeAt(offset)
      if (prior >= 0xd800 && prior <= 0xdbff && next >= 0xdc00 && next <= 0xdfff) {
        fail('E_SEMANTIC_PROVENANCE', path, 'UTF-16 span endpoint splits a surrogate pair')
      }
    }
  }
  program.provenance.forEach((record, index) => {
    assertScalarBoundary(record.span.start, `$.provenance[${index}].span.start`)
    assertScalarBoundary(record.span.end, `$.provenance[${index}].span.end`)
  })
  program.diagnostics.forEach((diagnostic, index) => {
    if (diagnostic.span === null) return
    assertScalarBoundary(diagnostic.span.start, `$.diagnostics[${index}].span.start`)
    assertScalarBoundary(diagnostic.span.end, `$.diagnostics[${index}].span.end`)
  })
}

export function semanticDiagnosticCodeIsPublic(value: string): value is SemanticProgramDiagnosticCode {
  return (SEMANTIC_PROGRAM_DIAGNOSTIC_CODES as readonly string[]).includes(value)
}
