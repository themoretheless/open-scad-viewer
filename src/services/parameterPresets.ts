import { sha256Hex } from '../core/sha256'
import { encodeCustomizerValue, extractCustomizerParameters, type CustomizerValue } from './scadCustomizer'

export const MAX_PARAMETER_PRESETS = 20
const MAX_PARAMETERS = 128
const MAX_PRESET_BYTES = 64 * 1024

export interface ParameterPreset {
  readonly id: string
  readonly name: string
  readonly templateHash: string
  readonly values: readonly { readonly name: string; readonly value: CustomizerValue }[]
}

export function parseParameterPresets(value: unknown): readonly ParameterPreset[] | null {
  if (!Array.isArray(value) || value.length > MAX_PARAMETER_PRESETS) return null
  const result: ParameterPreset[] = []
  for (const item of value) {
    if (!item || typeof item !== 'object'
      || typeof item.id !== 'string' || !item.id.length || item.id.length > 128
      || typeof item.name !== 'string' || !item.name.trim() || item.name.length > 80
      || typeof item.templateHash !== 'string' || !/^[a-f0-9]{64}$/.test(item.templateHash)
      || !Array.isArray(item.values) || !item.values.length || item.values.length > MAX_PARAMETERS) return null
    const values: Array<{ name: string; value: CustomizerValue }> = []
    for (const entry of item.values) {
      if (!entry || typeof entry !== 'object' || typeof entry.name !== 'string'
        || entry.name.length > 128 || !/^[A-Za-z_][A-Za-z0-9_$]*$/.test(entry.name)
        || !(typeof entry.value === 'boolean'
          || (typeof entry.value === 'number' && Number.isFinite(entry.value))
          || (typeof entry.value === 'string' && entry.value.length <= 2048))) return null
      values.push({ name: entry.name, value: entry.value })
    }
    if (new Set(values.map(entry => entry.name)).size !== values.length) return null
    result.push({ id: item.id, name: item.name.trim(), templateHash: item.templateHash, values })
  }
  if (new Set(result.map(item => item.id)).size !== result.length
    || new Set(result.map(item => item.name)).size !== result.length
    || new TextEncoder().encode(JSON.stringify(result)).byteLength > MAX_PRESET_BYTES) return null
  return result
}

/** Exact source template binding: only the captured top-level literal values may differ. */
export function parameterTemplateHash(source: string): string {
  const parameters = extractCustomizerParameters(source)
  const pieces: Array<string | { type: string }> = []
  let offset = 0
  for (const parameter of parameters) {
    pieces.push(source.slice(offset, parameter.valueStart), { type: typeof parameter.value })
    offset = parameter.valueEnd
  }
  pieces.push(source.slice(offset))
  return sha256Hex(JSON.stringify(pieces))
}

export function captureParameterPreset(source: string, name: string, id: string): ParameterPreset {
  const preset = {
    id, name: name.trim(), templateHash: parameterTemplateHash(source),
    values: extractCustomizerParameters(source).map(parameter => ({ name: parameter.name, value: parameter.value })),
  }
  const validated = parseParameterPresets([preset])
  if (!validated) throw new Error('preset_invalid')
  return validated[0]
}

export function applyParameterPreset(source: string, preset: ParameterPreset): string {
  if (!parseParameterPresets([preset]) || parameterTemplateHash(source) !== preset.templateHash) throw new Error('preset_incompatible')
  const parameters = extractCustomizerParameters(source)
  if (parameters.length !== preset.values.length) throw new Error('preset_incompatible')
  // Prepare every replacement before touching the caller's source.
  const replacements = parameters.map((parameter, index) => {
    const entry = preset.values[index]
    if (entry.name !== parameter.name || typeof entry.value !== typeof parameter.value) throw new Error('preset_incompatible')
    return { ...parameter, encoded: encodeCustomizerValue(entry.value) }
  })
  let result = source
  for (const parameter of replacements.reverse()) result = result.slice(0, parameter.valueStart) + parameter.encoded + result.slice(parameter.valueEnd)
  return result
}
