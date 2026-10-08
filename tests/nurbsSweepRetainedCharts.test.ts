import {expect,it} from 'vitest'
import {inspectSurfaceLinearInjectivity} from '../src/services/nurbsSweepAudit'
import {inspectSweepRetainedWallCharts} from '../src/services/nurbsSweepRetainedCharts'
import {createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
const quarter={degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],controlPoints:[[[1,0,0],[1,0,5]],[[1,1,0],[1,1,5]],[[0,1,0],[0,1,5]]],weights:[[1,1],[Math.sqrt(.5),Math.sqrt(.5)],[1,1]],periodicU:false,periodicV:false}
it('proves a full rational quarter chart with one oblique projection and refuses folds',()=>{
 const report=inspectSurfaceLinearInjectivity(quarter,1000)
 expect(report).toMatchObject({certified:true,projection:[[-1,1,0],[0,0,1]],reason:null,globalEmbeddingCertified:false})
 expect(report.cells).toBeGreaterThan(1)
 expect(inspectSurfaceLinearInjectivity(quarter,0)).toMatchObject({certified:false,cells:0,reason:'cell-budget-exhausted'})
 expect(inspectSurfaceLinearInjectivity(quarter,100,[[1,0,0],[0,0,1]]).certified).toBe(false)
 expect(inspectSurfaceLinearInjectivity(quarter,1000,[[-1,0,0],[0,0,1]]).certified).toBe(false)
 const folded={...structuredClone(quarter),degreeU:1,knotsU:[0,0,.5,1,1],weights:[[1,1],[1,1],[1,1]]}
 folded.controlPoints[2]=structuredClone(folded.controlPoints[0]!)
 expect(inspectSurfaceLinearInjectivity(folded,1000,[[-1,1,0],[0,0,1]]).certified).toBe(false)
 expect(()=>inspectSurfaceLinearInjectivity(quarter,100,[[1,0,0],[1,0,0]])).toThrow()
})
it('certifies actual decomposed hollow-loft walls with shared budget and face identity',()=>{
 const loops=(z:number)=>[[circleNurbsCurve([0,0,z],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,z],[0,0,1],.2))]]
 const model=createRationalBrepSectionLoft([loops(0),loops(5),loops(10)]),before=structuredClone(model)
 const report=inspectSweepRetainedWallCharts(model,[16,17],10000)
 expect(report).toMatchObject({allChartsCertified:true,unresolvedFaces:[],globalEmbeddingCertified:false})
 expect(report.charts.map(chart=>chart.face)).toEqual(Array.from({length:16},(_,i)=>i))
 expect(report.cells).toBeLessThanOrEqual(10000)
 const zero=inspectSweepRetainedWallCharts(model,[16,17],0)
 expect(zero).toMatchObject({allChartsCertified:false,cells:0,unresolvedFaces:Array.from({length:16},(_,i)=>i)})
 const short=inspectSweepRetainedWallCharts(model,[16,17],1)
 expect(short.allChartsCertified).toBe(false)
 expect(short.cells).toBeLessThanOrEqual(1)
 expect(()=>inspectSweepRetainedWallCharts(model,[16,17],1000,()=>{throw new DOMException('cancel','AbortError')})).toThrow(/cancel/)
 expect(model).toEqual(before)
})
