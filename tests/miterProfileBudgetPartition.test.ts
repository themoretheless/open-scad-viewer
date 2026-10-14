import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'

it('preserves station G2 through Rush and viewport after deferred profile work',()=>{
 const source=readFileSync(new URL('../examples/rush/sharp-profile-nonuniform-station-g2-miter.r',import.meta.url),'utf8')
 const document=compileRushFrontend(source).document
 const node=document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 const construction=built.report.construction![node.id]!
 if(!('profileSmoothness' in construction))throw new Error('Missing native miter smoothness report')
 const smooth=construction.profileSmoothness
 expect(smooth.maxWork).toBe(2000000)
 expect(smooth.station.maxWork).toBe(smooth.maxWork-smooth.exactWork)
 expect(smooth.station.exactWork).toBeLessThanOrEqual(smooth.station.maxWork)
 expect(smooth.totalExactWork).toBe(smooth.exactWork+smooth.station.exactWork)
 expect(smooth.totalExactWork).toBeLessThanOrEqual(smooth.maxWork)
 expect(smooth.station.stationG2Certified).toBe(true)
 expect(readSweepViewportEvidence(built.nativeGeometry)).toMatchObject({
  continuousBound:true,stationG1Certified:true,stationG2Certified:true,stationContinuity:'G2',
 })
})
