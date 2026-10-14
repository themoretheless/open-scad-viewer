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
import { compareBytes, encodeSemanticProgramCore } from './semanticProgramCodec/isSemanticProgramCodecOwnedGraph'
export { isSemanticProgramCodecOwnedGraph, encodeSemanticProgram, encodeSemanticProgramCore, semanticProgramHash, encodeSemanticTessellationPolicy, semanticTessellationPolicyHash, attestSemanticProgram, decodeSemanticProgram, decodeSemanticProgramCore, decodeSemanticTessellationPolicy } from './semanticProgramCodec/isSemanticProgramCodecOwnedGraph'

export const SEMANTIC_PROGRAM_BINARY_FORMAT = 'semantic-program-binary-v1' as const
export const SEMANTIC_PROGRAM_BINARY_MAX_BYTES = 64 * 1024 * 1024
export const SEMANTIC_PROGRAM_BINARY_MAX_STRING_BYTES = SEMANTIC_PROGRAM_LIMITS.canonicalStringBytes

export const ENVELOPE_MAGIC = new Uint8Array([0x53, 0x50, 0x45, 0x31]) // SPE1
export const CORE_MAGIC = new Uint8Array([0x53, 0x50, 0x43, 0x31]) // SPC1
export const TESSELLATION_MAGIC = new Uint8Array([0x54, 0x53, 0x50, 0x31]) // TSP1
export const PROGRAM_HASH_DOMAIN = new TextEncoder().encode('semantic-program-core-v1')
export const TESSELLATION_HASH_DOMAIN = new TextEncoder().encode('tessellation-policy-v1')

const CODEC_DECODED_GRAPHS = new WeakSet<object>()
export const addCodecDecodedGraph = CODEC_DECODED_GRAPHS.add.bind(CODEC_DECODED_GRAPHS)
export const hasCodecDecodedGraph = CODEC_DECODED_GRAPHS.has.bind(CODEC_DECODED_GRAPHS)

export const enum Tag {
  Null = 0,
  False = 1,
  True = 2,
  Number = 3,
  String = 4,
  Array = 5,
  Object = 6,
}

export interface BinaryWriter {
  byte(value: number): void
  u16(value: number): void
  u32(value: number): void
  f64(value: number): void
  raw(value: Uint8Array): void
}

export class ByteWriter implements BinaryWriter {
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

export class CanonicalBodyComparator implements BinaryWriter {
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

export interface EncodingBudget { stringBytes: number }

/** Backward-compatible API name; the identity bytes are exactly SPC1 core. */
export const encodeSemanticProgramIdentity = encodeSemanticProgramCore

export interface SemanticTessellationPolicyV1 {
  readonly schema: 'semantic-tessellation-policy'
  readonly schemaVersion: Readonly<{ major: 1; minor: 0 }>
  readonly programHash: string
  readonly intents: readonly SemanticTessellationIntent[]
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

export class ByteReader {
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
