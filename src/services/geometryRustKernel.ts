/** Synchronous host boundary for the own Rust geometry libraries; no geometry fallback. */
import { initSync, execute, SurfaceEvaluator } from '../generated/geometry-kernels/kernel.js'
import { gunzipSync } from 'fflate/browser'
import wasmBase64 from '../generated/geometry-kernels/bytes'

export class GeometryKernelError extends Error {
  constructor(public readonly code: string, message: string) { super(message); this.name = 'GeometryKernelError' }
}
export class NurbsCurveError extends GeometryKernelError {
  constructor(public readonly code: 'NURBS_INVALID_INPUT' | 'NURBS_RESOURCE_LIMIT' | 'NURBS_NUMERIC_ERROR', message: string) {
    super(code, message)
    this.name = 'NurbsCurveError'
  }
}
let initialized = false
function initialize(): void {
  if (initialized) return
  const binary = atob(wasmBase64)
  initSync({ module: gunzipSync(Uint8Array.from(binary, character => character.charCodeAt(0))) })
  initialized = true
}
export function decodeNurbsResult<T>(text: string): T {
  const result = JSON.parse(text)
  if (!result.ok) {
    const ErrorType = result.error.code.startsWith('NURBS_') ? NurbsCurveError : GeometryKernelError
    throw new ErrorType(result.error.code, result.error.message)
  }
  return result.value as T
}
export function callGeometryRust<T>(op: string, args: object): T {
  initialize()
  return decodeNurbsResult<T>(execute(JSON.stringify({ op, ...args })))
}
export function createRustSurfaceEvaluator(surface: object): SurfaceEvaluator {
  initialize()
  try { return new SurfaceEvaluator(JSON.stringify(surface)) }
  catch (error) {
    if (typeof error === 'string') {
      const detail = JSON.parse(error)
      throw new NurbsCurveError(detail.code, detail.message)
    }
    throw error
  }
}
