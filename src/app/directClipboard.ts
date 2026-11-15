import type { Vec3 } from '../kernel/geometry/math3d'
import { transformSelection } from './directSolidTools'
import { parseDirectDocument, serializeDirectDocument, type DirectDocument } from './directModeling'
import { documentUnit, isDocumentLengthUnit, toMillimetres, type DocumentLengthUnit } from './documentUnits'

/**
 * Copy with a base point between documents (roadmap C6). The clipboard carries a versioned
 * JSON payload: the copied bodies and sketches in kernel millimetres, the base point in the
 * source document's units and that unit. Pasting maps the base point onto a target point given
 * in the target document's units, so the geometry keeps its physical size across documents
 * with different units. NURBS curves and surfaces are not part of the payload yet.
 */
export const DIRECT_CLIPBOARD_FORMAT = 'open-scad-viewer/direct-objects'
export const DIRECT_CLIPBOARD_VERSION = 1
export const MAX_DIRECT_CLIPBOARD_CHARACTERS = 32 * 1024 * 1024

export interface DirectClipboardPayload {
  format: typeof DIRECT_CLIPBOARD_FORMAT
  version: typeof DIRECT_CLIPBOARD_VERSION
  units: DocumentLengthUnit
  basePoint: Vec3
  sketches: DirectDocument['sketches']
  bodies: DirectDocument['bodies']
}

const finiteVec3 = (value: unknown): value is Vec3 =>
  Array.isArray(value) && value.length === 3 && value.every(v => typeof v === 'number' && Number.isFinite(v))

export function createDirectClipboardPayload(
  document: DirectDocument,
  ids: readonly string[],
  basePoint: Vec3,
): string {
  if (!finiteVec3(basePoint)) throw Error('The base point must be three finite numbers.')
  const selected = new Set(ids)
  const sketches = document.sketches.filter(s => selected.has(s.id))
  // Linked instances need their source; copy them as independent bodies only when the source comes along.
  const bodies = document.bodies.filter(b => selected.has(b.id) && (!b.instance || selected.has(b.instance.sourceId)))
  if (!sketches.length && !bodies.length) throw Error('Select bodies or sketches to copy.')
  const payload: DirectClipboardPayload = {
    format: DIRECT_CLIPBOARD_FORMAT,
    version: DIRECT_CLIPBOARD_VERSION,
    units: documentUnit(document),
    basePoint: [...basePoint] as Vec3,
    sketches: [],
    bodies: [],
  }
  // Serialize through the document writer so meshes become plain JSON arrays.
  const objects = JSON.parse(serializeDirectDocument({ version: 1, sketches, bodies }))
  payload.sketches = objects.sketches
  payload.bodies = objects.bodies
  const text = JSON.stringify(payload)
  if (text.length > MAX_DIRECT_CLIPBOARD_CHARACTERS) throw Error('The selection is too large to copy.')
  return text
}

/** Validates the envelope; the objects themselves are validated when pasted into a document. */
export function parseDirectClipboardPayload(text: string): DirectClipboardPayload {
  if (typeof text !== 'string' || text.length > MAX_DIRECT_CLIPBOARD_CHARACTERS)
    throw Error('Clipboard data is too large.')
  let value: Partial<DirectClipboardPayload>
  try {
    value = JSON.parse(text) as Partial<DirectClipboardPayload>
  } catch {
    throw Error('The clipboard does not contain copied objects.')
  }
  if (!value || value.format !== DIRECT_CLIPBOARD_FORMAT) throw Error('The clipboard does not contain copied objects.')
  if (value.version !== DIRECT_CLIPBOARD_VERSION) throw Error('Copied objects come from an unsupported version.')
  if (
    !isDocumentLengthUnit(value.units) ||
    !finiteVec3(value.basePoint) ||
    !Array.isArray(value.sketches) ||
    !Array.isArray(value.bodies)
  )
    throw Error('Copied objects are malformed.')
  if (!value.sketches.length && !value.bodies.length) throw Error('Copied objects are malformed.')
  return value as DirectClipboardPayload
}

/**
 * Paste into `target` with fresh ids so that the copied base point lands on `targetPoint`
 * (in the target document's units). Returns the new document and the created ids.
 */
export function pasteDirectClipboardPayload(
  target: DirectDocument,
  payload: DirectClipboardPayload,
  targetPoint: Vec3,
  newId: () => string = () => crypto.randomUUID(),
): { document: DirectDocument; ids: string[] } {
  if (!finiteVec3(targetPoint)) throw Error('The target point must be three finite numbers.')
  const baseMm = payload.basePoint.map(v => toMillimetres(v, payload.units)) as Vec3
  const targetMm = targetPoint.map(v => toMillimetres(v, documentUnit(target))) as Vec3
  const delta = [0, 1, 2].map(k => targetMm[k] - baseMm[k]) as Vec3
  const taken = new Set(
    [
      ...target.sketches,
      ...target.bodies,
      ...(target.curves ?? []),
      ...(target.surfaces ?? []),
      ...(target.sourceBodies ?? []),
    ].map(o => o.id),
  )
  const ids = new Map<string, string>()
  for (const object of [...payload.sketches, ...payload.bodies]) {
    let id = newId()
    while (taken.has(id) || [...ids.values()].includes(id)) id = newId()
    ids.set(object.id, id)
  }
  const sketches = structuredClone(payload.sketches).map(s => ({ ...s, id: ids.get(s.id)! }))
  const bodies = structuredClone(payload.bodies).map(b => {
    const body = { ...b, id: ids.get(b.id)! }
    if (body.instance)
      body.instance = { ...body.instance, sourceId: ids.get(body.instance.sourceId) ?? body.instance.sourceId }
    return body
  })
  let next = parseDirectDocument(
    serializeDirectDocument({
      ...target,
      sketches: [...target.sketches, ...sketches],
      bodies: [...target.bodies, ...bodies],
    }),
  )
  const created = [...ids.values()]
  if (delta.some(v => v !== 0))
    next = parseDirectDocument(serializeDirectDocument(transformSelection(next, created, delta, [0, 0, 1], 0, 1)))
  return { document: next, ids: created }
}
