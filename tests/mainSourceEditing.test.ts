import {it,expect} from 'vitest'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {mainOperation} from '../src/services/mainModeling'
import {patchMainSource,restoreSourceHistory,boundedSourceHistory} from '../src/services/mainSourceEditing'
it('edits one top-level expression and preserves declarations, modules and unrelated primitives',async()=>{
 const source='// user note\nx=10;\nmodule spare(){ sphere(3); }\ncube(x);\ntranslate([30,0,0]) sphere(5);',m=(await parseOpenSCAD(source)).meshes,d=mainOperation(m,0,null,'move',{amount:0,x:2,y:0,z:0,axis:'x',edge:0,shape:'circle',width:1,height:1,cut:false}),next=patchMainSource(source,m,d)
 expect(next).toContain('// user note\nx=10;\nmodule spare(){ sphere(3); }');expect(next).toContain('translate([30,0,0]) sphere(5);');expect(next).toContain('polyhedron(');expect((await parseOpenSCAD(next)).meshes).toHaveLength(2)
})
it('restores only history matching the current source and respects storage budget',()=>{
 const d={past:[{before:'a',after:'b'}],future:[{before:'b',after:'c'}]};expect(restoreSourceHistory(JSON.stringify(d),'b')).toEqual(d);expect(restoreSourceHistory(JSON.stringify(d),'other').past).toEqual([]);expect(boundedSourceHistory({past:Array.from({length:100},()=>({before:'a',after:'b'})),future:[]}).past).toHaveLength(80)
})
it('retains native body and face identity under parameter changes',async()=>{const a=(await parseOpenSCAD('cube([10,10,10]);')).meshes[0],b=(await parseOpenSCAD('cube([10,10,20]);')).meshes[0];expect(a.entityId).toBe(b.entityId);expect([...new Set(a.faceIds)]).toEqual([...new Set(b.faceIds)])})
it('resolves an attached sketch plane after a body parameter changes',async()=>{const {resolveSketchSupport}=await import('../src/services/mainSketchAssociation');const a=(await parseOpenSCAD('cube([10,10,10]);')).meshes,b=(await parseOpenSCAD('cube([10,10,20]);')).meshes;const top=Array.from(a[0].faceIds).find((id,i)=>{try{return resolveSketchSupport(a,{entity:a[0].entityId!,face:id}).origin[2]===10}catch{return false}})!;expect(resolveSketchSupport(b,{entity:a[0].entityId!,face:top}).origin[2]).toBe(20);expect(()=>resolveSketchSupport([],{entity:a[0].entityId!,face:top})).toThrow('missing')})
