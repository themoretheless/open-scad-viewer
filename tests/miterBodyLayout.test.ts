import {expect,it} from 'vitest'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {planMiterBody,partitionMiterSections,ownedMiterSharpStations,previewMiterWalls} from '../src/services/miterBodyLayout'

const ring=(z:number,r:number)=>circleNurbsCurve([0,0,z],[0,0,1],r)
it('preserves the real face budget, cap reservation and refinements beyond 64',()=>{
 const loops=[[ring(0,1)],[ring(0,.25)]]
 expect(planMiterBody(loops,17,false,1,64)).toEqual({edges:16,spans:8,maxSteps:7})
 expect(planMiterBody(loops,16,true,1,64)).toEqual({edges:16,spans:8,maxSteps:8})
 expect(planMiterBody([[ring(0,1)]],2,false,1,256)).toEqual({edges:1,spans:4,maxSteps:255})
 expect(()=>planMiterBody(loops,17,false,8,64)).toThrow(/initial steps exceed face budget/)
 expect(()=>planMiterBody([[]],2,false,1,64)).toThrow(/nonempty loops/)
})
it('keeps hole partition and every original sharp vertex, including cyclic closure',()=>{
 const sections=Array.from({length:7},(_,z)=>[ring(z,1),ring(z,.25)])
 const before=structuredClone(sections),partition=partitionMiterSections(sections,[1,1])
 expect(partition[6]).toEqual([[sections[6]![0]],[sections[6]![1]]])
 expect(ownedMiterSharpStations(7,3,2,false)).toEqual([2,4])
 expect(ownedMiterSharpStations(7,3,2,true)).toEqual([0,2,4])
 expect(ownedMiterSharpStations(256,1,255,false)).toEqual([])
 expect(()=>ownedMiterSharpStations(6,3,2,true)).toThrow(/complete uniform span coverage/)
 expect(()=>partitionMiterSections(sections,[1])).toThrow(/complete ring coverage/)
 expect(sections).toEqual(before)
 partition[0]![0]![0]!.controlPoints[0]![0]+=100
 expect(sections).toEqual(before)
})
it('constructs the complete native hollow preview and refuses omitted profiles',()=>{
 const sections=Array.from({length:3},(_,z)=>[ring(z,1),ring(z,.25)])
 const before=structuredClone(sections),preview=previewMiterWalls(sections)
 expect(preview.patches).toHaveLength(16)
 expect(preview.profilePatchRanges).toEqual([[0,8],[8,16]])
 expect(preview.patches.every(p=>p.degreeV===1&&p.knotsV.join(',')==='0,0,1,1')).toBe(true)
 const missing=structuredClone(sections);missing[1]!.pop()
 expect(()=>previewMiterWalls(missing)).toThrow(/complete profile coverage/)
 expect(sections).toEqual(before)
})
