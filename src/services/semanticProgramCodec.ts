import { sha256Hex } from '../core/sha256'
import {
  SEMANTIC_PROGRAM_CONTRACT,
  SEMANTIC_PROGRAM_LIMITS,
  type SemanticProgramCoreV1,
  type SemanticTessellationIntent,
  type SemanticProgramV1,
} from '../core/semanticProgram'
import {
  normalizeSemanticProgramCoreInput,
  normalizeSemanticProgram,
  normalizeDecodedSemanticProgram,
  normalizeDecodedSemanticProgramCore,
  SemanticProgramValidationError,
} from './semanticProgramValidator'

export const SEMANTIC_PROGRAM_BINARY_FORMAT = 'semantic-program-binary-v1' as const
export const SEMANTIC_PROGRAM_BINARY_MAX_BYTES = 64 * 1024 * 1024
export const SEMANTIC_PROGRAM_BINARY_MAX_STRING_BYTES = SEMANTIC_PROGRAM_LIMITS.canonicalStringBytes

const ENVELOPE_MAGIC = new Uint8Array([0x53, 0x50, 0x45, 0x31]) // SPE1
const CORE_MAGIC = new Uint8Array([0x53, 0x50, 0x43, 0x31]) // SPC1
const TESSELLATION_MAGIC = new Uint8Array([0x54, 0x53, 0x50, 0x31]) // TSP1
const PROGRAM_HASH_DOMAIN = new TextEncoder().encode('semantic-program-core-v1')
const TESSELLATION_HASH_DOMAIN = new TextEncoder().encode('tessellation-policy-v1')

const CODEC_DECODED_GRAPHS = new WeakSet<object>()
const addCodecDecodedGraph = CODEC_DECODED_GRAPHS.add.bind(CODEC_DECODED_GRAPHS)
const hasCodecDecodedGraph = CODEC_DECODED_GRAPHS.has.bind(CODEC_DECODED_GRAPHS)

/** @internal Read-only half of the codec-owned graph capability. */
export function isSemanticProgramCodecOwnedGraph(value: unknown): boolean {
  return value !== null && typeof value === 'object' && hasCodecDecodedGraph(value)
}

const enum Tag {
  Null = 0,
  False = 1,
  True = 2,
  Number = 3,
  String = 4,
  Array = 5,
  Object = 6,
}

interface BinaryWriter {
  byte(value: number): void
  u16(value: number): void
  u32(value: number): void
  f64(value: number): void
  raw(value: Uint8Array): void
}

class ByteWriter implements BinaryWriter {
  private readonly chunks: Uint8Array[] = []
  private current = new Uint8Array(64 * 1024)
  private offset = 0
  private length = 0

  private reserve(length: number): void {
    if (this.length + length > SEMANTIC_PROGRAM_BINARY_MAX_BYTES) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', '$binary', 'canonical binary exceeds its byte limit')
  }

  byte(value: number): void {
    this.reserve(1)
    if (this.offset === this.current.length) {
      this.chunks.push(this.current)
      this.current = new Uint8Array(64 * 1024)
      this.offset = 0
    }
    this.current[this.offset++] = value & 0xff
    this.length++
  }

  u16(value: number): void {
    if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', '$binary', 'value exceeds u16')
    this.byte((value >>> 8) & 0xff)
    this.byte(value & 0xff)
  }

  u32(value: number): void {
    if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff_ffff) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', '$binary', 'value exceeds u32')
    this.byte((value >>> 24) & 0xff)
    this.byte((value >>> 16) & 0xff)
    this.byte((value >>> 8) & 0xff)
    this.byte(value & 0xff)
  }

  f64(value: number): void {
    const bytes = new Uint8Array(8)
    new DataView(bytes.buffer).setFloat64(0, value, true)
    this.raw(bytes)
  }

  raw(value: Uint8Array): void {
    this.reserve(value.length)
    let inputOffset = 0
    while (inputOffset < value.length) {
      if (this.offset === this.current.length) {
        this.chunks.push(this.current)
        this.current = new Uint8Array(64 * 1024)
        this.offset = 0
      }
      const count = Math.min(value.length - inputOffset, this.current.length - this.offset)
      this.current.set(value.subarray(inputOffset, inputOffset + count), this.offset)
      this.offset += count
      this.length += count
      inputOffset += count
    }
  }

  finish(): Uint8Array {
    const output = new Uint8Array(this.length)
    let outputOffset = 0
    for (const chunk of this.chunks) {
      output.set(chunk, outputOffset)
      outputOffset += chunk.length
    }
    output.set(this.current.subarray(0, this.offset), outputOffset)
    return output
  }
}

class CanonicalBodyComparator implements BinaryWriter {
  private offset = 12
  private matches = true

  constructor(private readonly expected: Uint8Array) {}

  byte(value: number): void {
    if (this.offset >= this.expected.length || this.expected[this.offset] !== (value & 0xff)) this.matches = false
    this.offset++
  }

  u16(value: number): void {
    if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff) {
      throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', '$binary', 'value exceeds u16')
    }
    this.byte((value >>> 8) & 0xff)
    this.byte(value & 0xff)
  }

  u32(value: number): void {
    if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff_ffff) {
      throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', '$binary', 'value exceeds u32')
    }
    this.byte((value >>> 24) & 0xff)
    this.byte((value >>> 16) & 0xff)
    this.byte((value >>> 8) & 0xff)
    this.byte(value & 0xff)
  }

  f64(value: number): void {
    const bytes = new Uint8Array(8)
    new DataView(bytes.buffer).setFloat64(0, value, true)
    this.raw(bytes)
  }

  raw(value: Uint8Array): void {
    for (const byte of value) this.byte(byte)
  }

  finish(): boolean {
    return this.matches && this.offset === this.expected.length
  }
}

function compareBytes(left: Uint8Array, right: Uint8Array): number {
  const shared = Math.min(left.length, right.length)
  for (let index = 0; index < shared; index++) if (left[index] !== right[index]) return left[index] - right[index]
  return left.length - right.length
}

interface EncodingBudget { stringBytes: number }

function addEncodedStringBytes(length: number, budget: EncodingBudget, path: string): void {
  if (length > SEMANTIC_PROGRAM_LIMITS.stringCodeUnits * 4) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', path, 'individual string exceeds its UTF-8 byte limit')
  budget.stringBytes += length
  if (budget.stringBytes > SEMANTIC_PROGRAM_BINARY_MAX_STRING_BYTES) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', path, 'aggregate string bytes exceed the encode budget')
}

function encodeStringBody(value: string, writer: BinaryWriter, budget: EncodingBudget, path: string): void {
  const bytes = new TextEncoder().encode(value)
  addEncodedStringBytes(bytes.length, budget, path)
  writer.u32(bytes.length)
  writer.raw(bytes)
}

function encodeValue(value: unknown, writer: BinaryWriter, budget: EncodingBudget, path = '$'): void {
  if (value === null) { writer.byte(Tag.Null); return }
  if (value === false) { writer.byte(Tag.False); return }
  if (value === true) { writer.byte(Tag.True); return }
  if (typeof value === 'number') {
    if (!Number.isFinite(value) || Object.is(value, -0)) throw new SemanticProgramValidationError('E_SEMANTIC_NUMBER', '$binary', 'only finite canonical binary64 values are encodable')
    writer.byte(Tag.Number)
    writer.f64(value)
    return
  }
  if (typeof value === 'string') {
    writer.byte(Tag.String)
    encodeStringBody(value, writer, budget, path)
    return
  }
  if (Array.isArray(value)) {
    writer.byte(Tag.Array)
    writer.u32(value.length)
    value.forEach((item, index) => encodeValue(item, writer, budget, `${path}[${index}]`))
    return
  }
  if (value !== null && typeof value === 'object') {
    writer.byte(Tag.Object)
    const entries = Object.entries(value as Record<string, unknown>)
      .map(([key, item]) => ({ item, bytes: new TextEncoder().encode(key) }))
      .sort((left, right) => compareBytes(left.bytes, right.bytes))
    writer.u32(entries.length)
    for (const entry of entries) {
      addEncodedStringBytes(entry.bytes.length, budget, `${path}.key`)
      writer.u32(entry.bytes.length)
      writer.raw(entry.bytes)
      encodeValue(entry.item, writer, budget, path)
    }
    return
  }
  throw new SemanticProgramValidationError('E_SEMANTIC_SCHEMA', '$binary', 'only JSON values are encodable')
}

function frame(magic: Uint8Array, payload: unknown): Uint8Array {
  const bodyWriter = new ByteWriter()
  encodeValue(payload, bodyWriter, { stringBytes: 0 })
  const body = bodyWriter.finish()
  const writer = new ByteWriter()
  writer.raw(magic)
  writer.u16(1)
  writer.u16(magic === TESSELLATION_MAGIC ? 0 : 2)
  writer.u32(body.length)
  writer.raw(body)
  return writer.finish()
}

export function encodeSemanticProgram(input: unknown): Uint8Array {
  const program = normalizeSemanticProgram(input)
  return frame(ENVELOPE_MAGIC, program)
}

export function encodeSemanticProgramCore(input: unknown): Uint8Array {
  const core = normalizeSemanticProgramCoreInput(input)
  return frame(CORE_MAGIC, core)
}

/** Backward-compatible API name; the identity bytes are exactly SPC1 core. */
export const encodeSemanticProgramIdentity = encodeSemanticProgramCore

function lengthPrefixedPreimage(domain: Uint8Array, payload: Uint8Array): Uint8Array {
  const output = new Uint8Array(4 + domain.length + 4 + payload.length)
  const view = new DataView(output.buffer)
  view.setUint32(0, domain.length, false)
  output.set(domain, 4)
  view.setUint32(4 + domain.length, payload.length, false)
  output.set(payload, 8 + domain.length)
  return output
}

export function semanticProgramHash(input: unknown): string {
  const core = normalizeSemanticProgramCoreInput(input)
  return semanticProgramHashFromCore(core)
}

function semanticProgramHashFromCore(core: SemanticProgramCoreV1, coreBytes = frame(CORE_MAGIC, core)): string {
  return sha256Hex(lengthPrefixedPreimage(PROGRAM_HASH_DOMAIN, coreBytes))
}

export interface SemanticTessellationPolicyV1 {
  readonly schema: 'semantic-tessellation-policy'
  readonly schemaVersion: Readonly<{ major: 1; minor: 0 }>
  readonly programHash: string
  readonly intents: readonly SemanticTessellationIntent[]
}

function tessellationPolicy(program: SemanticProgramV1, programHash: string): SemanticTessellationPolicyV1 {
  return {
    schema: 'semantic-tessellation-policy',
    schemaVersion: { major: 1, minor: 0 },
    programHash,
    intents: program.tessellationIntents,
  }
}

export function encodeSemanticTessellationPolicy(input: unknown): Uint8Array {
  const program = normalizeSemanticProgram(input)
  const coreBytes = frame(CORE_MAGIC, program.core)
  return frame(TESSELLATION_MAGIC, tessellationPolicy(program, semanticProgramHashFromCore(program.core, coreBytes)))
}

export function semanticTessellationPolicyHash(input: unknown): string {
  const program = normalizeSemanticProgram(input)
  const coreBytes = frame(CORE_MAGIC, program.core)
  const policy = frame(TESSELLATION_MAGIC, tessellationPolicy(program, semanticProgramHashFromCore(program.core, coreBytes)))
  return sha256Hex(lengthPrefixedPreimage(TESSELLATION_HASH_DOMAIN, policy))
}

export interface SemanticProgramAttestation {
  readonly semanticProgramVersion: typeof SEMANTIC_PROGRAM_CONTRACT
  readonly binaryFormat: typeof SEMANTIC_PROGRAM_BINARY_FORMAT
  readonly sourceHash: string
  readonly programHash: string
  readonly tessellationPolicyHash: string
  readonly coreBytesSha256: string
  readonly envelopeBytesSha256: string
  readonly canonicalBytes: number
}

export function attestSemanticProgram(input: unknown): SemanticProgramAttestation {
  const program = normalizeSemanticProgram(input)
  const envelopeBytes = frame(ENVELOPE_MAGIC, program)
  const coreBytes = frame(CORE_MAGIC, program.core)
  const programHash = semanticProgramHashFromCore(program.core, coreBytes)
  const policyBytes = frame(TESSELLATION_MAGIC, tessellationPolicy(program, programHash))
  return Object.freeze({
    semanticProgramVersion: SEMANTIC_PROGRAM_CONTRACT,
    binaryFormat: SEMANTIC_PROGRAM_BINARY_FORMAT,
    sourceHash: program.source.sha256,
    programHash,
    tessellationPolicyHash: sha256Hex(lengthPrefixedPreimage(TESSELLATION_HASH_DOMAIN, policyBytes)),
    coreBytesSha256: sha256Hex(coreBytes),
    envelopeBytesSha256: sha256Hex(envelopeBytes),
    canonicalBytes: envelopeBytes.length,
  })
}

class ByteReader {
  private offset = 0
  private values = 0
  private stringBytes = 0

  constructor(private readonly bytes: Uint8Array) {}
  get remaining(): number { return this.bytes.length - this.offset }

  raw(length: number, path: string): Uint8Array {
    if (!Number.isSafeInteger(length) || length < 0 || length > this.remaining) throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', path, 'truncated binary payload')
    const output = this.bytes.subarray(this.offset, this.offset + length)
    this.offset += length
    return output
  }

  byte(path: string): number { return this.raw(1, path)[0] }
  u16(path: string): number {
    const bytes = this.raw(2, path)
    return new DataView(bytes.buffer, bytes.byteOffset, 2).getUint16(0, false)
  }
  u32(path: string): number {
    const bytes = this.raw(4, path)
    return new DataView(bytes.buffer, bytes.byteOffset, 4).getUint32(0, false)
  }
  f64(path: string): number {
    const bytes = this.raw(8, path)
    const value = new DataView(bytes.buffer, bytes.byteOffset, 8).getFloat64(0, true)
    if (!Number.isFinite(value) || Object.is(value, -0)) throw new SemanticProgramValidationError('E_SEMANTIC_NUMBER', path, 'noncanonical binary64 encoding')
    return value
  }

  stringBody(path: string): { value: string; bytes: Uint8Array } {
    const length = this.u32(`${path}.length`)
    if (length > SEMANTIC_PROGRAM_LIMITS.stringCodeUnits * 4) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', path, 'individual string exceeds its UTF-8 byte limit')
    this.stringBytes += length
    if (this.stringBytes > SEMANTIC_PROGRAM_BINARY_MAX_STRING_BYTES) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', path, 'aggregate string bytes exceed the decode budget')
    const bytes = this.raw(length, path)
    let value: string
    try {
      value = new TextDecoder('utf-8', { fatal: true }).decode(bytes)
    } catch {
      throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', path, 'invalid or truncated UTF-8')
    }
    if (compareBytes(bytes, new TextEncoder().encode(value)) !== 0) throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', path, 'non-shortest UTF-8')
    return { value, bytes }
  }

  value(path = '$', depth = 0): unknown {
    if (++this.values > SEMANTIC_PROGRAM_LIMITS.snapshotValues) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', path, 'binary value budget exceeded')
    if (depth > SEMANTIC_PROGRAM_LIMITS.snapshotDepth) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', path, 'binary depth exceeded')
    const tag = this.byte(`${path}.tag`)
    switch (tag) {
      case Tag.Null: return null
      case Tag.False: return false
      case Tag.True: return true
      case Tag.Number: return this.f64(path)
      case Tag.String: return this.stringBody(path).value
      case Tag.Array: {
        const length = this.u32(`${path}.length`)
        if (length > SEMANTIC_PROGRAM_LIMITS.snapshotValues || length > this.remaining) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', path, 'array count exceeds available bytes or value budget')
        const output: unknown[] = []
        for (let index = 0; index < length; index++) output.push(this.value(`${path}[${index}]`, depth + 1))
        return output
      }
      case Tag.Object: {
        const length = this.u32(`${path}.length`)
        if (length > SEMANTIC_PROGRAM_LIMITS.snapshotValues || length > Math.floor(this.remaining / 5)) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', path, 'object count exceeds available bytes or value budget')
        const output: Record<string, unknown> = Object.create(null)
        let previous: Uint8Array | null = null
        for (let index = 0; index < length; index++) {
          const key = this.stringBody(`${path}.key[${index}]`)
          if (previous !== null && compareBytes(previous, key.bytes) >= 0) throw new SemanticProgramValidationError('E_SEMANTIC_ORDER', path, 'object keys are duplicate or not in canonical UTF-8 byte order')
          previous = key.bytes
          Object.defineProperty(output, key.value, {
            value: this.value(`${path}.${key.value}`, depth + 1),
            enumerable: true,
            configurable: true,
            writable: true,
          })
        }
        return output
      }
      default:
        throw new SemanticProgramValidationError('E_SEMANTIC_VERSION', `${path}.tag`, `unknown binary tag ${tag}`)
    }
  }
}

function equalBytes(left: Uint8Array, right: Uint8Array): boolean {
  return left.length === right.length && left.every((byte, index) => byte === right[index])
}

function decodeFramedValue(input: Uint8Array, magic: Uint8Array, label: string): {
  readonly canonicalInput: Uint8Array
  readonly decoded: unknown
} {
  if (!(input instanceof Uint8Array)) throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', '$binary', 'expected Uint8Array')
  if (!(input.buffer instanceof ArrayBuffer)) throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', '$binary', 'shared binary buffers are forbidden')
  if (input.length > SEMANTIC_PROGRAM_BINARY_MAX_BYTES) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', '$binary', 'binary exceeds its byte limit')
  // Reject an invalid fixed header before allocating a full adversarial copy.
  if (input.length < 12) throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', '$binary', 'truncated binary frame')
  if (!magic.every((byte, index) => input[index] === byte)) {
    throw new SemanticProgramValidationError('E_SEMANTIC_VERSION', '$binary.magic', `expected ${label}`)
  }
  const header = new DataView(input.buffer, input.byteOffset, input.byteLength)
  const expectedMinor = magic === TESSELLATION_MAGIC ? 0 : 2
  const headerMajor = header.getUint16(4, false)
  const headerMinor = header.getUint16(6, false)
  if (headerMajor === 1 && expectedMinor === 2 && headerMinor < 2) {
    throw new SemanticProgramValidationError(
      'E_SEMANTIC_RELOWER_REQUIRED',
      '$binary.version',
      'SPE/SPC frames before 1.2 require exact-source re-lowering',
    )
  }
  if (headerMajor !== 1 || headerMinor !== expectedMinor) {
    throw new SemanticProgramValidationError('E_SEMANTIC_VERSION', '$binary.version', 'unsupported binary schema version')
  }
  if (header.getUint32(8, false) !== input.length - 12) {
    throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', '$binary.length', 'framed length does not match exact bytes')
  }
  // Decoding is synchronous and invokes no caller callbacks, so a non-shared
  // ArrayBuffer can be borrowed safely. Avoiding an eager full-frame copy keeps
  // malformed valid-header inputs at O(1) additional binary memory.
  const reader = new ByteReader(input)
  const actualMagic = reader.raw(4, '$binary.magic')
  if (!equalBytes(actualMagic, magic)) throw new SemanticProgramValidationError('E_SEMANTIC_VERSION', '$binary.magic', `expected ${label}`)
  const bodyMajor = reader.u16('$binary.major')
  const bodyMinor = reader.u16('$binary.minor')
  if (bodyMajor !== 1 || bodyMinor !== expectedMinor) throw new SemanticProgramValidationError('E_SEMANTIC_VERSION', '$binary.version', 'unsupported binary schema version')
  const bodyLength = reader.u32('$binary.length')
  if (bodyLength !== reader.remaining) throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', '$binary.length', 'framed length does not match exact bytes')
  const decoded = reader.value()
  if (reader.remaining !== 0) throw new SemanticProgramValidationError('E_SEMANTIC_BYTES', '$binary', 'trailing bytes are forbidden')
  if (decoded !== null && typeof decoded === 'object') addCodecDecodedGraph(decoded)
  return { canonicalInput: input, decoded }
}

function canonicalBodyEquals(input: Uint8Array, value: unknown): boolean {
  const comparator = new CanonicalBodyComparator(input)
  encodeValue(value, comparator, { stringBytes: 0 })
  return comparator.finish()
}

export function decodeSemanticProgram(input: Uint8Array): SemanticProgramV1 {
  const { canonicalInput, decoded } = decodeFramedValue(input, ENVELOPE_MAGIC, 'SPE1')
  const program = normalizeDecodedSemanticProgram(decoded)
  if (!canonicalBodyEquals(canonicalInput, program)) throw new SemanticProgramValidationError('E_SEMANTIC_ORDER', '$binary', 'payload is not canonical')
  return program
}

export function decodeSemanticProgramCore(input: Uint8Array): SemanticProgramCoreV1 {
  const { canonicalInput, decoded } = decodeFramedValue(input, CORE_MAGIC, 'SPC1')
  const core = normalizeDecodedSemanticProgramCore(decoded)
  if (!canonicalBodyEquals(canonicalInput, core)) throw new SemanticProgramValidationError('E_SEMANTIC_ORDER', '$binary', 'payload is not canonical')
  return core
}

function exactPolicyKeys(value: unknown, expected: readonly string[], path: string): asserts value is Record<string, unknown> {
  if (value === null || Array.isArray(value) || typeof value !== 'object') throw new SemanticProgramValidationError('E_SEMANTIC_SCHEMA', path, 'expected an object')
  const actual = Object.keys(value).sort()
  const wanted = [...expected].sort()
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new SemanticProgramValidationError('E_SEMANTIC_FIELD', path, 'unknown or missing fields')
  }
}

function positivePolicyNumber(value: unknown, path: string): void {
  if (typeof value !== 'number' || !Number.isFinite(value) || Object.is(value, -0) || value <= 0) {
    throw new SemanticProgramValidationError('E_SEMANTIC_NUMBER', path, 'expected a finite positive number')
  }
}

function policySegment(value: unknown, path: string): void {
  if (!Number.isSafeInteger(value) || (value as number) < 3 || (value as number) > 1_000_000) {
    throw new SemanticProgramValidationError('E_SEMANTIC_NUMBER', path, 'expected a segment count from 3 to 1000000')
  }
}

function freezePolicy<T>(value: T): T {
  if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.values(value as Record<string, unknown>).forEach(freezePolicy)
    Object.freeze(value)
  }
  return value
}

function normalizeDecodedPolicy(value: unknown): SemanticTessellationPolicyV1 {
  exactPolicyKeys(value, ['schema', 'schemaVersion', 'programHash', 'intents'], '$policy')
  if (value.schema !== 'semantic-tessellation-policy') throw new SemanticProgramValidationError('E_SEMANTIC_SCHEMA', '$policy.schema', 'expected semantic-tessellation-policy')
  exactPolicyKeys(value.schemaVersion, ['major', 'minor'], '$policy.schemaVersion')
  if (value.schemaVersion.major !== 1 || value.schemaVersion.minor !== 0) throw new SemanticProgramValidationError('E_SEMANTIC_VERSION', '$policy.schemaVersion', 'only schema version 1.0 is supported')
  if (typeof value.programHash !== 'string' || !/^[a-f0-9]{64}$/.test(value.programHash)) throw new SemanticProgramValidationError('E_SEMANTIC_FIELD', '$policy.programHash', 'expected lowercase SHA-256')
  if (!Array.isArray(value.intents) || value.intents.length > SEMANTIC_PROGRAM_LIMITS.tessellationIntents) throw new SemanticProgramValidationError('E_SEMANTIC_LIMIT', '$policy.intents', 'invalid tessellation intent count')
  let priorOccurrence = -1
  value.intents.forEach((intent, index) => {
    const path = `$.policy.intents[${index}]`
    exactPolicyKeys(intent, ['occurrence', 'chordTolerance', 'angularToleranceDegrees', 'minSegments', 'maxSegments'], path)
    if (!Number.isSafeInteger(intent.occurrence) || (intent.occurrence as number) < 0 || (intent.occurrence as number) >= SEMANTIC_PROGRAM_LIMITS.occurrences) throw new SemanticProgramValidationError('E_SEMANTIC_REFERENCE', `${path}.occurrence`, 'invalid occurrence index')
    if ((intent.occurrence as number) <= priorOccurrence) throw new SemanticProgramValidationError('E_SEMANTIC_ORDER', `${path}.occurrence`, 'intents must be unique and ordered')
    priorOccurrence = intent.occurrence as number
    if (intent.chordTolerance !== null) positivePolicyNumber(intent.chordTolerance, `${path}.chordTolerance`)
    if (intent.angularToleranceDegrees !== null) positivePolicyNumber(intent.angularToleranceDegrees, `${path}.angularToleranceDegrees`)
    if (intent.minSegments !== null) policySegment(intent.minSegments, `${path}.minSegments`)
    if (intent.maxSegments !== null) policySegment(intent.maxSegments, `${path}.maxSegments`)
    if (intent.minSegments !== null && intent.maxSegments !== null && (intent.maxSegments as number) < (intent.minSegments as number)) {
      throw new SemanticProgramValidationError('E_SEMANTIC_NUMBER', path, 'maxSegments cannot be below minSegments')
    }
  })
  return freezePolicy(value as unknown as SemanticTessellationPolicyV1)
}

export function decodeSemanticTessellationPolicy(
  input: Uint8Array,
  expectedProgram?: unknown,
): SemanticTessellationPolicyV1 {
  const { canonicalInput, decoded } = decodeFramedValue(input, TESSELLATION_MAGIC, 'TSP1')
  const policy = normalizeDecodedPolicy(decoded)
  if (!canonicalBodyEquals(canonicalInput, policy)) throw new SemanticProgramValidationError('E_SEMANTIC_ORDER', '$binary', 'payload is not canonical')
  if (expectedProgram !== undefined) {
    const program = normalizeSemanticProgram(expectedProgram)
    const expectedHash = semanticProgramHashFromCore(program.core)
    if (policy.programHash !== expectedHash) throw new SemanticProgramValidationError('E_SEMANTIC_IDENTITY', '$policy.programHash', 'tessellation policy is bound to a different program')
    if (program.core.language.contract === 'legacy/current' && policy.intents.length !== 0) {
      throw new SemanticProgramValidationError('E_SEMANTIC_TYPE', '$policy.intents', 'legacy/current cannot carry tessellation intents')
    }
    if (policy.intents.length !== program.tessellationIntents.length
      || policy.intents.some((intent, index) => {
        const expected = program.tessellationIntents[index]
        return intent.occurrence !== expected.occurrence
          || intent.chordTolerance !== expected.chordTolerance
          || intent.angularToleranceDegrees !== expected.angularToleranceDegrees
          || intent.minSegments !== expected.minSegments
          || intent.maxSegments !== expected.maxSegments
      })) {
      throw new SemanticProgramValidationError('E_SEMANTIC_IDENTITY', '$policy.intents', 'tessellation policy does not exactly match the source-bound program envelope')
    }
    policy.intents.forEach((intent, index) => {
      if (intent.occurrence >= program.core.occurrences.length || program.core.occurrences[intent.occurrence].node === null) {
        throw new SemanticProgramValidationError('E_SEMANTIC_REFERENCE', `$.policy.intents[${index}].occurrence`, 'policy references a non-producing occurrence')
      }
    })
  }
  return policy
}
