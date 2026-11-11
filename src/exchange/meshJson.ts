/**
 * JSON boundary for documents that embed PolygonMesh typed arrays.
 *
 * JSON.stringify(Float64Array) produces an object (`{"0": ...}`), which breaks
 * every document validator downstream. All serialization of mesh-bearing
 * documents (history commits, drafts, downloads, share payloads) must go
 * through `stringifyMeshJson`, which emits numeric views as plain arrays —
 * the exact text shape documents had before the typed-array migration.
 *
 * Parse side needs no reviver: schema validators (parseDirectDocument,
 * parseMeshDocument, …) box plain arrays back into Float64Array/Uint32Array
 * at the single field they own.
 */
const isNumericView = (v: unknown): v is ArrayBufferView & ArrayLike<number> =>
  ArrayBuffer.isView(v) && !(v instanceof DataView) && !(v instanceof BigInt64Array) && !(v instanceof BigUint64Array)

export function stringifyMeshJson(value: unknown, space?: string | number): string {
  return JSON.stringify(value, (_key, v) => (isNumericView(v) ? Array.from(v as ArrayLike<number>) : v), space)
}

/** Deep clone for mesh-bearing documents: structuredClone copies typed arrays
 * in O(1) without the per-element walk of the old JSON.parse(JSON.stringify()). */
export function cloneMeshDocument<T>(value: T): T {
  return structuredClone(value)
}
