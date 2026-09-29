import { callGeometryRust } from './geometry/kernel'
export type QuantityKind = 'length' | 'angle' | 'scalar'
export type QuantityResult = { valid: true; value: number } | { valid: false; reason: 'number' | 'unit' | 'range' }
export function parseCadQuantity(text: string, kind: QuantityKind, min?: number, max?: number): QuantityResult {
  return callGeometryRust('cad_quantity', { text, kind, min, max })
}
