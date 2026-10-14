import assert from 'node:assert/strict'
import {readFile, readdir, mkdir, writeFile, mkdtemp, rm} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import {join, resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {parseArgs} from 'node:util'
import {build, preview} from 'vite'
import {chromium} from 'playwright'
const {values} = parseArgs({options: {'cancel-checkpoint': {type: 'string'}, output: {type: 'string'}}})
const output = resolve(values.output || process.env.COOPERATIVE_BVH_OUTPUT || 'tmp/qualification/cooperative-bvh')
const cancelAt = Number(values['cancel-checkpoint'] || process.env.ANALYSIS_CANCEL_CHECKPOINT || 80)
assert.ok(Number.isSafeInteger(cancelAt) && cancelAt > 0)
const scratch = await mkdtemp(join(tmpdir(), 'osv-cooperative-bvh-'))
let server, browser
try {
  await build({configFile:false, publicDir:false, base:'/__cooperative/', worker:{format:'es'}, build:{outDir:scratch,
    lib:{entry:resolve('tools/browser-qualification/cooperative-bvh.ts'),formats:['es'],fileName:()=> 'harness.js'}}})
  const files = new Map()
  async function scan(path, prefix='') {
    for (const entry of await readdir(path, {withFileTypes:true})) {
      const key=prefix+entry.name
      if(entry.isDirectory()) await scan(join(path,entry.name),key+'/')
      else files.set(key,join(path,entry.name))
    }
  }
  await scan(scratch)
  server = await preview({preview:{host:'127.0.0.1',port:4186,strictPort:true}})
  browser = await chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE ? {executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
  const page=await browser.newPage(), pageErrors=[]
  page.on('pageerror', error=>pageErrors.push(error.message))
  await page.route('**/__cooperative/**', route=> {
    const key=new URL(route.request().url()).pathname.slice('/__cooperative/'.length)
    const path=files.get(key)
    return path ? route.fulfill({contentType:'text/javascript',path}) : route.fulfill({status:404,body:'missing harness asset'})
  })
  await page.route('**/__cooperative-test', route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Cooperative BVH qualification</title>'}))
  await page.goto('http://127.0.0.1:4186/__cooperative-test')
  const result=await page.evaluate(async checkpoint => (await import('/__cooperative/harness.js')).run(checkpoint), cancelAt)
  assert.deepEqual(pageErrors,[])
  assert.equal(result.cancelled.length,4)
  const hashes={}
  for(const file of ['crates/polygon-core/src/solid/bvh.rs','crates/polygon-core/src/solid/edges.rs','crates/mesh-topology/src/edges/cooperative.rs','crates/mesh-topology/src/edges.rs','crates/mesh-query/src/bvh.rs','crates/geometry-bridge/src/mesh_analysis.rs','crates/geometry-bridge/src/mesh.rs','crates/geometry-bridge/src/abi.rs','crates/geometry-wasm/src/lib.rs','src/services/geometry/meshAnalysis.ts','src/services/cadKernelOps.ts','src/services/openscadParser.ts','public/wasm/geometry-kernel.wasm','tools/browser-qualification/cooperative-bvh.ts','tools/browser-qualification/cooperative-bvh.worker.ts','tools/browser-qualification/cooperative-bvh.mjs']) {
    hashes[file]=createHash('sha256').update(await readFile(file)).digest('hex')
  }
  await mkdir(output,{recursive:true})
  await writeFile(join(output,'browser.json'),JSON.stringify({timestamp:new Date().toISOString(),cancelAt,browser:browser.version(),hashes,pageErrors,...result},null,2)+'\n')
  console.log(JSON.stringify({output,workerStarts:result.workerStarts,cancelled:result.cancelled,recovered:result.recovered,completed:result.completed},null,2))
} finally { try {await browser?.close()} finally {try {await server?.close()}finally {await rm(scratch,{recursive:true,force:true})}} }
