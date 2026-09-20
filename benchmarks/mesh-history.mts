import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {pathToFileURL} from 'node:url'
import {resolve} from 'node:path'
import {MeshHistory, type MeshWorkspaceDocument} from '../src/services/meshEditing'

assert.ok(process.argv[2], 'Pass the baseline meshEditing.ts path')
const baselineUrl = pathToFileURL(resolve(process.argv[2]))
const {MeshHistory: Baseline} = await import(baselineUrl.href) as {MeshHistory: typeof MeshHistory}
const hash = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex')
const median = (values: number[]) => [...values].sort((a,b) => a-b)[Math.floor(values.length/2)]
const samples = []
for (const triangles of [1_000, 10_000]) for (const initialCommits of [0, 80]) {
  const positions: number[] = [], indices: number[] = []
  for (let t=0;t<triangles;t++) {
    const x=t%100,y=Math.floor(t/100)
    positions.push(x,y,0,x+1,y,0,x,y+1,0)
    indices.push(t*3,t*3+1,t*3+2)
  }
  const document: MeshWorkspaceDocument = {version:1, objects:[{id:'mesh',name:'seed',visible:true,mesh:{positions,indices}}]}
  const baseline = new Baseline(document), candidate = new MeshHistory(document)
  for (let i=0;i<initialCommits;i++) {
    document.objects[0].name=`initial-${i}`
    baseline.commit(document); candidate.commit(document)
  }
  const before: number[] = [], after: number[] = []
  for (let i=0;i<9;i++) {
    document.objects[0].name=`sample-${i}`
    for (const current of i%2?[true,false]:[false,true]) {
      const start=performance.now()
      ;(current?candidate:baseline).commit(document)
      ;(current?after:before).push(performance.now()-start)
    }
    assert.deepEqual(candidate.document,baseline.document)
  }
  const historyNames: string[] = []
  while (baseline.canUndo) {
    assert.equal(candidate.canUndo,true)
    const expected=baseline.undo(),actual=candidate.undo()
    assert.deepEqual(actual,expected)
    historyNames.push(actual.objects[0].name)
  }
  assert.equal(candidate.canUndo,false)
  while (baseline.canRedo) assert.deepEqual(candidate.redo(),baseline.redo())
  assert.equal(candidate.canRedo,false)
  samples.push({triangles,initialCommits,retainedUndo:historyNames.length,documentCharacters:JSON.stringify(document).length,
    beforeMedianMs:median(before),afterMedianMs:median(after),before,after,
    historySha256:hash(Buffer.from(JSON.stringify(historyNames)))})
}
console.log(JSON.stringify({node:process.version,arch:process.arch,
  baselineSha256:hash(readFileSync(baselineUrl)),candidateSha256:hash(readFileSync(new URL('../src/services/meshEditing.ts',import.meta.url))),
  method:'Nine alternating commits after 0 or 80 setup commits; each changes only the name on an otherwise unchanged mesh. Initial empty histories grow during sampling; filled histories may evict by character/count limits. Validation, snapshot allocation and GC included; fixture setup and full document/undo/redo parity checks excluded.',samples},null,2))
