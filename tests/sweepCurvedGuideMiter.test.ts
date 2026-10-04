import {it,expect} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
it('combines a rational curved guide with reparameterized authored frame and affine laws',()=>{
 const source=readFileSync('examples/rush/miter-rational-curved-guide-frame-affine-hollow.r','utf8')
 const graph=compileModelGraphText(source)
 const node=graph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(graph.document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 expect(built.report.construction![node.id]).toMatchObject({authoredFramesApplied:true,orientationGuideApplied:true,affineLawsApplied:true,continuousBound:true,volume:{solidGeometryCertified:true}})
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({profileG2Certified:true,continuousBound:true,solidGeometryCertified:true})
})
