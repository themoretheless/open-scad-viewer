import {describe,it,expect} from 'vitest'
import {setLiveBodyPattern,detachLiveBodyPattern} from '../src/services/liveBodyPattern'
import {resolveSolidInstances,transformSolidInstance,detachSolidInstances} from '../src/services/solidInstances'
import {emptyDirectDocument,DirectHistory,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'

function seed(size=1){const d=emptyDirectDocument(),brep=createBrepBox([0,0,0],[size,2,3]);d.bodies.push({id:'source',name:'Source',brep,mesh:tessellateNurbsBrep(brep,1)});return d}
describe('live body patterns through native placements',()=>{
 it('keeps IDs stable, regenerates source geometry and changes count atomically',()=>{
  const original=seed(),grid={kind:'grid' as const,rows:2,columns:3,spacing:[10,20] as [number,number]}
  const first=resolveSolidInstances(setLiveBodyPattern(original,'source',grid))
  expect(first.bodies).toHaveLength(6);expect(original.bodies).toHaveLength(1)
  expect(first.bodies[3].instance!.matrix[1][3]).toBe(20)
  const edited=structuredClone(first);edited.bodies[0]={...seed(4).bodies[0],livePattern:grid}
  const updated=resolveSolidInstances(edited)
  expect(updated.bodies.map(b=>b.id)).toEqual(first.bodies.map(b=>b.id))
  expect(inspectPolygonMesh(updated.bodies[1].mesh).signedVolumeMm3).toBeCloseTo(24)
  const smaller=resolveSolidInstances(setLiveBodyPattern(updated,'source',{...grid,columns:2}))
  expect(smaller.bodies).toHaveLength(4)
  expect(smaller.bodies[1].id).toEqual(first.bodies[1].id)
  expect(resolveSolidInstances(smaller).bodies).toHaveLength(4)
 })
 it('retains B-rep for radial and reflected copies and survives compact save and Undo',()=>{
  const radial=resolveSolidInstances(setLiveBodyPattern(seed(),'source',{kind:'radial',count:4,axis:'z',center:[5,0,0]}))
  expect(radial.bodies).toHaveLength(4);expect(radial.bodies.every(b=>b.brep)).toBe(true)
  const mirrored=resolveSolidInstances(setLiveBodyPattern(radial,'source',{kind:'mirror',axis:'x',center:[5,0,0]}))
  expect(mirrored.bodies).toHaveLength(2)
  expect(inspectPolygonMesh(mirrored.bodies[1].mesh).signedVolumeMm3).toBeCloseTo(6)
  const restored=parseDirectDocument(serializeDirectDocument(mirrored))
  expect(restored.bodies.map(b=>b.id)).toEqual(mirrored.bodies.map(b=>b.id))
  expect(restored.bodies[0].livePattern).toEqual(mirrored.bodies[0].livePattern)
  const history=new DirectHistory(radial);history.commit(mirrored)
  expect(history.undo().bodies).toHaveLength(4);expect(history.redo().bodies).toHaveLength(2)
 })
 it('rejects invalid metadata and protects derived members until the whole pattern is detached',()=>{
  const patterned=resolveSolidInstances(setLiveBodyPattern(seed(),'source',{kind:'grid',rows:2,columns:2,spacing:[10,10]}))
  expect(()=>transformSolidInstance(patterned,patterned.bodies[1].id,[1,0,0],[0,0,1],0,1)).toThrow(/pattern/)
  expect(()=>detachSolidInstances(patterned,[patterned.bodies[1].id])).toThrow(/pattern/)
  expect(()=>resolveSolidInstances({...patterned,bodies:patterned.bodies.slice(1)})).toThrow(/source/)
  expect(()=>resolveSolidInstances(setLiveBodyPattern(seed(),'source',{kind:'grid',rows:256,columns:256,spacing:[1,1]}))).toThrow()
  const detached=resolveSolidInstances(detachLiveBodyPattern(patterned,'source'))
  expect(detached.bodies).toHaveLength(4);expect(detached.bodies.every(b=>!b.instance)).toBe(true)
  expect(detached.bodies[0].livePattern).toBeUndefined()
  expect(resolveSolidInstances(setLiveBodyPattern(patterned,'source',null)).bodies).toHaveLength(1)
 })
})
