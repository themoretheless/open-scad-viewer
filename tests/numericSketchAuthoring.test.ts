import {beforeAll,it,expect} from 'vitest'
import {numericRectangle,numericSlotSources} from '../src/services/numericSketchAuthoring'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {prepareSolidProfile} from '../src/services/solidProfilePreparation'
import {xyPlane} from '../src/services/directSketchGeometry'
beforeAll(warmGeometryKernel)
it('bounds rectangle corners, dimensions and non-finite input',()=>{
 expect(numericRectangle([20,-5],[10,6])).toEqual([[20,-5],[30,-5],[30,1],[20,1]])
 for(const [point,size] of [[[0,0],[0,10]],[[0,0],[NaN,10]],[[999999,0],[2,1]],[[0,0],[1e6+1,10]]])expect(numericRectangle(point as [number,number],size as [number,number])).toBeNull()
})
it.each([[0,0,10,0],[10,0,0,0],[0,0,3,4],[3,4,0,0]])('retains slot lines and rational end caps for centers %s,%s,%s,%s',(x0,y0,x1,y1)=>{
 const input=numericSlotSources([x0,y0],[x1,y1],4,xyPlane())!
 const {report,document}=prepareSolidProfile(input.document,input.ids,1e-7)
 expect(report.accepted,report.detail).toBe(true);expect(document.sketches).toHaveLength(1)
 const profile=document.sketches[0].retainedProfile!
 expect(profile.loops[0].filter(c=>c.degree===1)).toHaveLength(2);expect(profile.loops[0].filter(c=>c.degree===2)).toHaveLength(4)
 expect(profile.areaMm2).toBeCloseTo(Math.hypot(x1-x0,y1-y0)*4+4*Math.PI,9)
 expect(profile.loops[0].some(c=>c.weights?.some(w=>w!==1))).toBe(true)
})
it('refuses coincident centers, bad width and contours beyond the coordinate limit',()=>{
 for(const [a,b,width] of [[[0,0],[0,0],4],[[0,0],[1e-7,0],4],[[0,0],[10,0],.01],[[0,0],[10,0],Infinity],[[999999,0],[999990,0],4]])expect(numericSlotSources(a as [number,number],b as [number,number],width as number,xyPlane())).toBeNull()
})
