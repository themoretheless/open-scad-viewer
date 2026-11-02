// Diagnostic-only module substitution; never edits the production provider.
import {defineConfig} from 'vitest/config'
import base from '../../vitest.config'
import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
const pack=process.env.CANDIDATE_PACK_DIR
const wasm=process.env.CANDIDATE_WASM_PATH
const expected=process.env.CANDIDATE_SHA256
if(!pack||!wasm||!expected)throw Error('Explicit candidate pack, WASM and hash are required')
const bytes=readFileSync(wasm)
if(createHash('sha256').update(bytes).digest('hex')!==expected)throw Error('Candidate digest mismatch')
const text=readFileSync(resolve(pack,'identity.ts'),'utf8')
if(!text.includes(expected)||!text.includes(String(bytes.length)))throw Error('Private packed identity mismatch')
const candidate={name:'isolated-candidate-node-tests',enforce:'pre' as const,
  load(id:string){for(const file of ['identity.ts','bytes.ts'])if(id.split('?')[0]===resolve('src/generated/geometry-kernels',file))return readFileSync(resolve(pack!,file),'utf8')},
}
export default defineConfig({...base,plugins:[candidate,...base.plugins!],test:{...base.test,maxWorkers:2}})
