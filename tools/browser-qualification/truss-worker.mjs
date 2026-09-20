import assert from 'node:assert/strict'
import {readFile,readdir,mkdir,writeFile,mkdtemp} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import {join,resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {build,preview} from 'vite'
import {chromium} from 'playwright'

const output=resolve(process.env.TRUSS_WORKER_OUTPUT||'tmp/performance/truss-worker')
const scratch=await mkdtemp(join(tmpdir(),'osv-truss-worker-'))
await build({configFile:false,publicDir:false,build:{outDir:scratch,
  lib:{entry:resolve('tools/browser-qualification/truss-worker.ts'),formats:['es'],fileName:()=> 'harness.js'},
}})
const workers=(await readdir('dist/assets')).filter(f=>/^mainSolid\.worker-.*\.js$/.test(f))
assert.equal(workers.length,1)
let server,browser
try {
  server=await preview({preview:{host:'127.0.0.1',port:0}})
  const origin=server.resolvedUrls.local[0]
  browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
  const page=await browser.newPage(),errors=[]
  page.on('pageerror',error=>errors.push(error.message))
  await page.route('**/__truss-harness.js',route=>route.fulfill({contentType:'text/javascript',path:join(scratch,'harness.js')}))
  await page.route('**/__truss-bench',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Truss worker qualification</title>'}))
  await page.goto(new URL('__truss-bench',origin).href)
  const result=await page.evaluate(async ({worker,warmups})=>(await import('/__truss-harness.js')).run(`/assets/${worker}`,warmups),
    {worker:workers[0],warmups:Number(process.env.TRUSS_WARMUPS??200)})
  assert.deepEqual(errors,[])
  const hashes={}
  for(const file of [`dist/assets/${workers[0]}`,'public/wasm/geometry-kernel.wasm','tools/browser-qualification/truss-worker.ts'])
    hashes[file]=createHash('sha256').update(await readFile(file)).digest('hex')
  await mkdir(output,{recursive:true})
  await writeFile(join(output,'report.json'),JSON.stringify({browser:browser.version(),hashes,...result},null,2)+'\n')
  console.log(JSON.stringify({...result,reports:result.reports.map(({samplesMs,...row})=>row)},null,2))
} finally {try {await browser?.close()}finally {await server?.close()}}
