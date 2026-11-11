/** Compare the font-consuming native PDF path before/after lossless storage. */
import assert from 'node:assert/strict'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { resolve } from 'node:path'
import { encodeBinary, decodeBinary } from '../src/core/valueBinaryCodec.ts'
const [before, after, directory] = process.argv.slice(2)
assert(before && after && directory)
const svg = "<svg xmlns='http://www.w3.org/2000/svg' width='297mm' height='210mm' viewBox='0 0 297 210'><path d='M10 10 L20 20' fill='none' stroke='black'/><text x='10' y='30'>Latin 123 Деталь</text></svg>"
async function pdf(path: string) {
  const bytes = readFileSync(path), module = await WebAssembly.compile(bytes)
  assert.deepEqual(WebAssembly.Module.imports(module), [])
  const instance = await WebAssembly.instantiate(module, {})
  const w = instance.exports as any
  const data = encodeBinary({ op: 'drawing_pdf', sheets: [svg, svg] }), ptr = w.abi_alloc(data.length)
  new Uint8Array(w.memory.buffer,ptr,data.length).set(data)
  const start=performance.now()
  try {
    const packed=w.abi_request(0,ptr,data.length), p=Number(packed&0xffffffffn), n=Number(packed>>32n)
    try {
      const reply:any=decodeBinary(new Uint8Array(w.memory.buffer,p,n))
      assert.equal(reply.ok,true,JSON.stringify(reply.error))
      assert.equal(typeof reply.value,'string')
      assert(reply.value.includes('/FontFile2') && reply.value.includes('/Count 2') && reply.value.includes('0414'))
      return { content: reply.value as string, elapsedMs: performance.now()-start, wasmSha256:createHash('sha256').update(bytes).digest('hex') }
    } finally {w.abi_free(p,n)}
  } finally {w.abi_free(ptr,data.length)}
}
const a=await pdf(before), b=await pdf(after)
assert.equal(b.content,a.content,'Font storage changed actual Unicode PDF bytes')
mkdirSync(directory,{recursive:true})
writeFileSync(resolve(directory,'unicode-font.pdf'),b.content)
const report={passed:true,pdfSha256:createHash('sha256').update(b.content).digest('hex'),pdfBytes:Buffer.byteLength(b.content),
  baseline:{wasmSha256:a.wasmSha256,elapsedMs:a.elapsedMs},candidate:{wasmSha256:b.wasmSha256,elapsedMs:b.elapsedMs},
  scope:'Actual native Unicode PDF bytes match; this is not a whole-engine performance benchmark.'}
writeFileSync(resolve(directory,'report.json'),JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report))
