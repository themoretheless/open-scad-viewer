import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {cpus} from 'node:os'
import {pathToFileURL} from 'node:url'
import {resolve} from 'node:path'
import * as candidate from '../src/services/meshSelectionOverlay'
import {identity} from '../src/services/math3d'

assert.ok(process.argv[2], 'Pass the baseline meshSelectionOverlay.ts path')
const baselineUrl = pathToFileURL(resolve(process.argv[2]))
const baseline: typeof candidate = await import(baselineUrl.href)
const hash = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex')
const median = (values: number[]) => [...values].sort((a,b) => a-b)[Math.floor(values.length/2)]
const samples = []
for (const side of [1, 32, 100]) {
  const vertices = new Float32Array((side+1)**2 * 6)
  for (let y=0;y<=side;y++) for (let x=0;x<=side;x++) {
    const offset=(y*(side+1)+x)*6
    vertices[offset]=x; vertices[offset+1]=y
  }
  const indices=new Uint32Array(side*side*6)
  for (let y=0;y<side;y++) for (let x=0;x<side;x++) {
    const a=y*(side+1)+x,b=a+1,c=a+side+1,d=c+1
    indices.set([a,b,c,b,d,c],(y*side+x)*6)
  }
  const faceIds=new Uint32Array(indices.length/3),transform=identity()
  const faceIndex=candidate.buildFaceTriangleIndex(faceIds,faceIds.length)
  const run=(module:typeof candidate)=>module.buildFaceOverlayGeometry(vertices,indices,faceIds,transform,0,0,20_000,faceIndex)
  const expected=run(baseline)
  assert.deepEqual(run(candidate),expected)
  for(let i=0;i<5;i++){run(baseline);run(candidate)}
  const repeats=side===1?1000:1
  const before:number[]=[],after:number[]=[]
  for(let sample=0;sample<15;sample++) {
    for(const current of sample%2?[true,false]:[false,true]) {
      const module=current?candidate:baseline
      const start=performance.now()
      let result=expected
      for(let i=0;i<repeats;i++)result=run(module)
      ;(current?after:before).push((performance.now()-start)/repeats)
      assert.deepEqual(result,expected)
    }
  }
  samples.push({triangles:faceIds.length,repeats,beforeMedianMs:median(before),afterMedianMs:median(after),before,after,
    trianglesSha256:hash(new Uint8Array(expected.triangles.buffer)),boundarySha256:hash(new Uint8Array(expected.boundaryLines.buffer))})
}
console.log(JSON.stringify({node:process.version,cpu:cpus()[0]?.model,
  baselineSha256:hash(readFileSync(baselineUrl)),candidateSha256:hash(readFileSync(new URL('../src/services/meshSelectionOverlay.ts',import.meta.url))),
  method:'Coplanar grids, one selected face; cached CSR index; 5 warmups and 15 alternating baseline/candidate samples. Exact output parity outside timing. Allocation and GC included; no GPU/rendering or index construction.',samples},null,2))
