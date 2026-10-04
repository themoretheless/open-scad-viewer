import {readFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {fileURLToPath} from 'node:url'
import {resolve} from 'node:path'
import {verifyPackedWasmChunk,verifyRawWasm} from './verify-packed-wasm.mjs'

/** Artifact provenance for STEP fixtures; this does not attest compiler inputs. */
export function sweepStepArtifactProvenance() {
 const root=fileURLToPath(new URL('../',import.meta.url))
 const read=file=>readFileSync(resolve(root,file))
 const wasm=read('src/generated/geometry-kernels/kernel_bg.wasm')
 const sha256=createHash('sha256').update(wasm).digest('hex')
 const identitySource=read('src/generated/geometry-kernels/identity.ts').toString('utf8')
 const match=/export default Object\.freeze\((\{[^\n]+\})\)/.exec(identitySource)
 if(!match)throw new Error('Geometry artifact identity format is invalid')
 const identity=JSON.parse(match[1])
 if(identity.sha256!==sha256||identity.byteLength!==wasm.length)throw new Error('Geometry artifact identity differs from STEP runtime binary')
 verifyRawWasm(read('public/wasm/geometry-kernel.wasm'),wasm,'STEP geometry public WASM')
 verifyPackedWasmChunk(read('src/generated/geometry-kernels/bytes.ts').toString('utf8'),wasm,'STEP geometry packed WASM')
 return {schema:'sweep-step-artifact/1',geometryWasmSha256:sha256,geometryWasmByteLength:wasm.length,publicAndPackedVerified:true}
}
