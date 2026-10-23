import {expect,it} from 'vitest'
import {createRuledSketchLoft,analyzeNurbsBrep} from '../src/services/geometry/brep'
import {retainedLoftFixture} from './support/retainedLoftFixture'
it('preserves rational contours and holes in a scaled translated ruled loft with the expected volume',async()=>{
 const sketches=await retainedLoftFixture(),before=structuredClone(sketches)
 const result=createRuledSketchLoft(sketches,['base','top'])
 expect(result.brep.bodies).toHaveLength(1)
 expect(result.brep.faces.filter(face=>face.holes.length===1)).toHaveLength(2)
 expect(result.brep.faces.some(face=>face.surface.weights.flat().some(w=>w!==1))).toBe(true)
 expect(analyzeNurbsBrep(result.brep).signedVolumeMm3).toBeCloseTo(560*Math.PI/3,6)
 expect(sketches).toEqual(before)
})
it('places loft sections in a common tilted basis including tangential plane displacement',async()=>{
 const sketches=await retainedLoftFixture()
 sketches[0].plane={origin:[11,13,17],u:[0,1,0],v:[0,0,1]}
 sketches[1].plane={origin:[21,15,20],u:[0,1,0],v:[0,0,1]}
 const result=createRuledSketchLoft(sketches,['base','top'])
 expect(result.brep.vertices.some(vertex=>vertex.point.every((v,i)=>Math.abs(v-[21,26,27][i])<1e-10))).toBe(true)
 expect(analyzeNurbsBrep(result.brep).signedVolumeMm3).toBeCloseTo(560*Math.PI/3,6)
})
it('refuses unsupported correspondence and section order without changing source sections',async()=>{
 const sketches=await retainedLoftFixture(),before=structuredClone(sketches)
 expect(()=>createRuledSketchLoft(sketches,['top','base'])).toThrow(/positive sketch normal/)
 expect(()=>createRuledSketchLoft(sketches,['base','top','base'])).toThrow(/two corresponding/)
 const invalid=structuredClone(sketches);invalid[1].retainedProfile!.loops[0][0].controlPoints[1][0]+=1
 expect(()=>createRuledSketchLoft(invalid,['base','top'])).toThrow(/corresponding/)
 expect(sketches).toEqual(before)
})
