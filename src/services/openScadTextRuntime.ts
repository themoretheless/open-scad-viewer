import createHarfBuzzModule from 'harfbuzzjs/hb.js'
import bindHarfBuzz from 'harfbuzzjs/hbjs.js'
import wasmBase64 from '../generated/harfbuzz/bytes'
import { unpackBrotliWasmBase64 } from './wasmBrotliPacking'

/** Loaded on demand; static payload imports share identical main/worker chunks. */
export async function createTextRuntime(): Promise<unknown> {
  const module = await createHarfBuzzModule({ wasmBinary: unpackBrotliWasmBase64(wasmBase64) })
  return bindHarfBuzz(module)
}
