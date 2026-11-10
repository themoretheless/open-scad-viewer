import {isGcodePreviewDocument, type GcodePreviewDocument, type GcodePreviewResponse} from './gcodePreviewProtocol'
import {GCODE_MAX_MOVE_ROWS, GCODE_MOVE_ROW_WIDTH, unpackGcodePreview} from './gcodePreviewTransport'

type TransferDocument = Omit<GcodePreviewDocument, 'preview'> & {
  preview: Omit<GcodePreviewDocument['preview'], 'moves'> & {moveRows: Float64Array<ArrayBuffer>}
}
export type GcodePreviewWireResponse = GcodePreviewResponse | {
  version: 1; id: number; ok: true; responseFormat: 'f64-moves-v1'; result: TransferDocument
}

/** Only this newly allocated output buffer is transferred; scene inputs remain owned by the caller. */
export function prepareGcodePreviewTransfer(response: GcodePreviewResponse): {
  response: GcodePreviewWireResponse; transfer: ArrayBuffer[]
} {
  if (!response.ok) return {response, transfer: []}
  const {moves, ...metadata} = response.result.preview
  if (moves.length > GCODE_MAX_MOVE_ROWS / GCODE_MOVE_ROW_WIDTH) throw new Error('Invalid packed G-code moves')
  const moveRows = new Float64Array(moves.length * GCODE_MOVE_ROW_WIDTH)
  for (let i = 0, j = 0; i < moves.length; i++, j += GCODE_MOVE_ROW_WIDTH) {
    const move = moves[i]
    moveRows[j] = move.x; moveRows[j + 1] = move.y; moveRows[j + 2] = move.z
    moveRows[j + 3] = move.e; moveRows[j + 4] = move.feedrateMmS
    moveRows[j + 5] = move.layerIndex; moveRows[j + 6] = move.extruded ? 1 : 0
  }
  return {response: {...response, responseFormat: 'f64-moves-v1',
    result: {...response.result, preview: {...metadata, moveRows}}}, transfer: [moveRows.buffer]}
}

/** Validate the bounded wire representation before allocating the public move objects. */
export function decodeGcodePreviewResponse(response: unknown): GcodePreviewDocument {
  const invalid = () => new Error('Invalid response from G-code processing.')
  if (!response || typeof response !== 'object') throw invalid()
  const wire = response as {responseFormat?: unknown; result?: unknown}
  if (wire.responseFormat === undefined) {
    if (!isGcodePreviewDocument(wire.result)) throw invalid()
    return wire.result
  }
  if (wire.responseFormat !== 'f64-moves-v1' || !wire.result || typeof wire.result !== 'object') throw invalid()
  const document = wire.result as TransferDocument, preview = document.preview
  if (!preview || typeof preview !== 'object' || 'moves' in preview) throw invalid()
  const rows = preview.moveRows
  if (!(rows instanceof Float64Array) || !(rows.buffer instanceof ArrayBuffer)
    || rows.byteOffset !== 0 || rows.byteLength !== rows.buffer.byteLength
    || rows.length > GCODE_MAX_MOVE_ROWS || rows.length % GCODE_MOVE_ROW_WIDTH !== 0
    || !isGcodePreviewDocument({...document, preview: {...preview, moves: []}})) throw invalid()
  try { return {...document, preview: unpackGcodePreview(preview)} }
  catch { throw invalid() }
}
