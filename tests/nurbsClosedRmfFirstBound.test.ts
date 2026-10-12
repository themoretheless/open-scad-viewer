import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepPatchViewportEvidence} from '../src/services/sweepViewportEvidence'

// Transport qualification only: original geometry and interval ownership stay
// native. This is the same unchanged tolerance/section ceiling as the Rust
// cyclic regression, including spatial quintic transitions.
it.each([
 ['round',false],['round',true],['transition',false],['transition',true],
] as const)('carries closed %s spatial=%s RMF first bounds through WASM/Rush/viewport',async(kind,spatial)=>{
 const source=`// @rush/1
profile = bezier_curve(points: [[0,8mm,0.25mm],[0,8mm,0.5mm]])
path = ${kind}_polyline_curve(points: [[0,0,0],[10mm,0,0],[10mm,10mm,${spatial?'5mm':'0'}],[0,10mm,0]],${kind==='round'?'radius':'setback'}: 2mm,closed: true)
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],orientation: "rmf",initial_sections: 5,max_sections: 257,max_deviation: 0.01mm
)
`
 const document=compileRushFrontend(source).document
 const node=document.nodes.find(n=>n.op==='progressive_sweep')!
 const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const report=built.report.construction![node.id]
 expect(report).toMatchObject({accepted:true,closedPath:true,continuousBound:true,roundingCertified:true})
 expect(report.continuousErrorUpper).toBeLessThanOrEqual(0.01)
 expect(readSweepPatchViewportEvidence(built.nativeGeometry)).toMatchObject({continuousBound:true})
 await expect(buildOwnNurbsAsync(compileRushFrontend(source.replace('max_sections: 257','max_sections: 5')).document,{action:'build',display:{segments:4,subdivisionLevels:0}})).rejects.toThrow(/Progressive sweep (?:continuous retained-patch error|sampled refinement)/)
})
