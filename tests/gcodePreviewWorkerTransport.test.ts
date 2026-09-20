import {expect, it} from 'vitest'
import type {GcodePreviewDocument, GcodePreviewResponse} from '../src/services/gcodePreviewProtocol'
import {decodeGcodePreviewResponse, prepareGcodePreviewTransfer} from '../src/services/gcodePreviewWorkerTransport'

const document = (): GcodePreviewDocument => ({
  gcode: '; file', dialect: 'test', native: false, generator: 'test', flavor: null,
  gcode3mfBase64: 'UEs=', preview: {
    layers: 1, extrusionMm: 1, depositedVolumeMm3: 2, travelDistanceMm: 0,
    printDistanceMm: 10, estimatedTimeS: 1, bounds: {min: [-0, 0, 0], max: [10, 1, 1]},
    moves: [{x: -0, y: 1.23456789012345, z: .2, e: .01, feedrateMmS: 5, layerIndex: 0, extruded: true}],
  },
})
const success = (): GcodePreviewResponse => ({version: 1, id: 7, ok: true, result: document()})

it('transfers only an owned output buffer, preserves complete documents and double precision', () => {
  const source = success(), original = structuredClone(source)
  const prepared = prepareGcodePreviewTransfer(source)
  expect(prepared.transfer).toHaveLength(1)
  expect(prepared.transfer[0].byteLength).toBe(7 * 8)
  const received = structuredClone(prepared.response, {transfer: prepared.transfer})
  expect(prepared.transfer[0].byteLength).toBe(0)
  expect(decodeGcodePreviewResponse(received)).toEqual(document())
  expect(source).toEqual(original)
  expect(decodeGcodePreviewResponse(received).preview).not.toHaveProperty('moveRows')
})

it('supports empty previews, legacy documents and unchanged failure envelopes', () => {
  const source = success()
  if (!source.ok) throw new Error('Expected success')
  expect(decodeGcodePreviewResponse(source)).toBe(source.result)
  source.result.preview.moves = []
  const prepared = prepareGcodePreviewTransfer(source)
  expect(decodeGcodePreviewResponse(structuredClone(prepared.response, {transfer: prepared.transfer}))).toEqual(source.result)
  const failure = {version: 1 as const, id: 3, ok: false as const, error: 'Parse error'}
  expect(prepareGcodePreviewTransfer(failure)).toEqual({response: failure, transfer: []})
})

it('rejects malformed, partial, oversized, shared and nonfinite wire rows', () => {
  const {moves, ...metadata} = document().preview
  const decode = (moveRows: unknown, changes = {}) => decodeGcodePreviewResponse({
    responseFormat: 'f64-moves-v1', result: {...document(), preview: {...metadata, moveRows, ...changes}},
  })
  for (const rows of [null, [], new Float32Array(7), new Float64Array(1), new Float64Array(700_007),
    new Float64Array(14).subarray(7), new Float64Array(new SharedArrayBuffer(56))]) {
    expect(() => decode(rows)).toThrow('Invalid response')
  }
  for (let i = 0; i < 7; i++) for (const value of [NaN, Infinity, -Infinity]) {
    const rows = new Float64Array(7); rows[i] = value
    expect(() => decode(rows)).toThrow('Invalid response')
  }
  for (const [column, value] of [[5, -1], [5, .5], [5, 2048], [6, 2]]) {
    const rows = new Float64Array(7); rows[column] = value
    expect(() => decode(rows)).toThrow('Invalid response')
  }
  for (const changes of [{layers: -1}, {extrusionMm: NaN}, {moves}, {bounds: {min: [], max: []}}]) {
    expect(() => decode(new Float64Array(7), changes)).toThrow('Invalid response')
  }
  for (const response of [null, {}, {responseFormat: 'f64-moves-v2', result: document()},
    {responseFormat: 'f64-moves-v1', result: document()}]) {
    expect(() => decodeGcodePreviewResponse(response)).toThrow('Invalid response')
  }
})

it('rejects excessive output before allocating its transfer buffer', () => {
  const source = success()
  if (!source.ok) throw new Error('Expected success')
  source.result.preview.moves = new Array(100_001)
  expect(() => prepareGcodePreviewTransfer(source)).toThrow('Invalid packed')
})
