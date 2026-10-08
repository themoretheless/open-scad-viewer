import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs,buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
const request={action:'build' as const,display:{segments:4,subdivisionLevels:0}}
it.each(['closed-profile-planar-10.r','closed-profile-spatial-6.r'])('forwards native retained-body proof through Rush, async build, viewport and Solid: %s',async file=>{
 const source=readFileSync(`examples/rush/${file}`,'utf8')
 const graph=compileRushFrontend(source).document
 if(file==='closed-profile-planar-10.r'){
  const tight=compileRushFrontend(source.replace('max_deviation:256mm','max_deviation:1mm')).document
  expect(()=>buildOwnNurbs(tight,request)).toThrow(/budget/)
 }
 const node=graph.nodes.find(n=>n.op==='brep_progressive_sweep')!
 for(const built of [buildOwnNurbs(graph,request),await buildOwnNurbsAsync(graph,request)]){
  expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({nodeId:node.id,solidGeometryCertified:true,wallRegularityCertified:true,continuousBound:false})
  const report=built.report.construction![node.id] as any
  expect(report.seamContinuity).toBe('C0')
  expect(report.volume.solidGeometryCertified).toBe(true)
  const artifact=built.nativeGeometry!
  const model=JSON.parse(artifact.geometryJson).geometry
  expect(inspectProgressiveSweepSolidAdmission(artifact,model)).toMatchObject({solidGeometryCertified:true})
 }
})
