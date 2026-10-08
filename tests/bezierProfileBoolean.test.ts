import {it,expect} from 'vitest'
import {pathFromCircle} from '../src/services/geometry/path2d'
import {authorBrepProfile,booleanBrepProfiles,signedAreaBrepProfileLoop,transformBrepProfile} from '../src/services/geometry/brepProfile'
import {combineSketchProfiles} from '../src/services/retainedSketchProfile'
import {withEditableSketchPath} from '../src/services/editableSketchPath'
it('keeps original cubic curves through transverse union, intersection and difference',()=>{
 const a=authorBrepProfile({kind:'bezier',path:pathFromCircle([0,0],5)}),b=authorBrepProfile({kind:'bezier',path:pathFromCircle([3,0],5)})
 const u=booleanBrepProfiles(a,b,'union'),i=booleanBrepProfiles(a,b,'intersection'),d=booleanBrepProfiles(a,b,'difference')
 expect(u.areaMm2+i.areaMm2).toBeCloseTo(2*a.areaMm2,5);expect(d.areaMm2+i.areaMm2).toBeCloseTo(a.areaMm2,5)
 for(const p of [u,i,d]){expect(p.loops.flat().every(c=>c.degree===3)).toBe(true);expect(signedAreaBrepProfileLoop(p.loops[0])).toBeCloseTo(p.areaMm2,5)}
 const scaled=transformBrepProfile(a,[2,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);expect(scaled.areaMm2).toBeCloseTo(2*a.areaMm2,5)
 const sketches=[0,3].map((x,n)=>withEditableSketchPath({id:'s'+n,name:'Curve',closed:true,points:[]},pathFromCircle([x,0],5)))
 const before=JSON.stringify(sketches),combined=combineSketchProfiles(sketches,'union')
 expect(combined.retainedProfile?.loops.flat().every(c=>c.degree===3)).toBe(true);expect(JSON.stringify(sketches)).toBe(before)
})
it('keeps external tangent components and their original carriers',()=>{
 const a=authorBrepProfile({kind:'bezier',path:pathFromCircle([0,0],5)}),b=authorBrepProfile({kind:'bezier',path:pathFromCircle([10,0],5)})
 const u=booleanBrepProfiles(a,b,'union');expect(u.loops).toHaveLength(2);expect(u.areaMm2).toBeCloseTo(2*a.areaMm2,5)
 expect(booleanBrepProfiles(a,b,'intersection').loops).toHaveLength(0)
 expect(booleanBrepProfiles(a,b,'difference').areaMm2).toBeCloseTo(a.areaMm2,5)
})

it('retains rational arcs in mixed cubic booleans and normalizes a crossing authored path',()=>{
 const a=authorBrepProfile({kind:'bezier',path:pathFromCircle([0,0],5)})
 const b=transformBrepProfile(authorBrepProfile({kind:'circle',radius:5}),[1,0,0,0,0,1,0,0,0,0,1,0,3,0,0,1])
 const u=booleanBrepProfiles(a,b,'union'),i=booleanBrepProfiles(a,b,'intersection'),d=booleanBrepProfiles(a,b,'difference')
 for(const p of [u,i,d]){expect(p.loops.flat().some(c=>c.degree===2)).toBe(true);expect(p.loops.flat().some(c=>c.degree===3)).toBe(true)}
 expect(u.areaMm2+i.areaMm2).toBeCloseTo(a.areaMm2+b.areaMm2,5);expect(d.areaMm2+i.areaMm2).toBeCloseTo(a.areaMm2,5)
 const crossing=authorBrepProfile({kind:'bezier',path:{start:[0,0],closed:true,segments:[{type:'cubic',c1:[4/3,4/3],c2:[8/3,8/3],to:[4,4]},{type:'line',to:[0,4]},{type:'line',to:[4,0]},{type:'line',to:[0,0]}]}})
 expect(crossing.loops).toHaveLength(2);expect(crossing.areaMm2).toBeCloseTo(8,5)
})
