/** Warm local comparison of owned solid analysis. No event-loop sleeps: the
 * cooperative timings include ABI polls/checkpoints but exclude scheduling delay.
 * node --import tsx benchmarks/cooperative-analysis.mts --out /tmp/analysis.json
 */
import {strict as assert} from 'node:assert'
import {createHash} from 'node:crypto'
import {readFileSync, writeFileSync} from 'node:fs'
import {parseArgs} from 'node:util'
import {CadGeometryKernel} from '../src/services/cadGeometryKernel.ts'
import {analyzeSolidCooperativelyInKernel, analyzeSolidInKernel} from '../src/services/geometry/meshAnalysis.ts'

const {values} = parseArgs({options: {out: {type: 'string'}}})
const session = await new CadGeometryKernel().openSession()
const rows = []
try {
  for (const segments of [32,128,256]) {
    const solid = session.module.CadSolid.sphere(3,segments)
    try {
      const reference = analyzeSolidInKernel(solid.handle)
      const sync: number[] = [], cooperative: number[] = []
      let checkpoints = 0
      for (let run = 0; run < 7; run++) {
        const measureSync = () => {
          const start = performance.now()
          const result = analyzeSolidInKernel(solid.handle)
          const elapsed = performance.now()-start
          assert.deepEqual(result,reference)
          if (run >= 2) sync.push(elapsed)
        }
        const measureCooperative = async () => {
          checkpoints = 0
          const start = performance.now()
          const result = await analyzeSolidCooperativelyInKernel(solid.handle,async()=>{checkpoints++})
          const elapsed = performance.now()-start
          assert.deepEqual(result,reference)
          if (run >= 2) cooperative.push(elapsed)
        }
        if (run % 2) { await measureCooperative(); measureSync() }
        else { measureSync(); await measureCooperative() }
      }
      const median = (samples: number[]) => [...samples].sort((a,b)=>a-b)[Math.floor(samples.length/2)]
      rows.push({segments,triangles:reference.mesh.indices.length/3,checkpoints,byteParity:true,
        syncMs:sync,cooperativeMs:cooperative,syncMedianMs:median(sync),cooperativeMedianMs:median(cooperative)})
    } finally { solid.delete() }
  }
} finally {session.dispose()}
const hashes = Object.fromEntries([
  'public/wasm/geometry-kernel.wasm','benchmarks/cooperative-analysis.mts',
  'src/services/geometry/meshAnalysis.ts','crates/geometry-bridge/src/mesh_analysis.rs',
  'crates/polygon-core/src/solid/edges.rs','crates/polygon-core/src/solid/edges/cooperative.rs',
].map(path=>[path,createHash('sha256').update(readFileSync(path)).digest('hex')]))
const report={timestamp:new Date().toISOString(),node:process.version,hashes,warmups:2,samples:5,
  scope:'Warm alternating local analysis calls; no event-loop sleeps or production cancellation latency claim',rows}
const json=JSON.stringify(report,null,2)+'\n'
if(values.out) writeFileSync(values.out,json,{flag:'wx'})
else console.log(json)
