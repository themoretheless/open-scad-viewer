import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {DirectHistory,parseDirectDocument,type DirectBody} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep,booleanNurbsBrep,inspectNurbsBrep,type NurbsBrep} from '../src/services/geometry/brep'
import {transformSelection,pushPullFace,solidTopology,splitSolid} from '../src/services/directSolidTools'
import {stringifyMeshJson} from '../src/services/meshJson'
const body=(brep:NurbsBrep,id='part'):DirectBody=>{const mesh=tessellateNurbsBrep(brep,2);return {id,name:'Part',brep,mesh:{positions:mesh.positions,indices:mesh.indices}}}
it('preserves topology identities through 20 mixed geometry edits, full history traversal and serialized restore',()=>{
 const history=new DirectHistory({version:1,sketches:[],bodies:[body(createBrepBox([0,0,0],[10,10,10]))]})
 const snapshots=[stringifyMeshJson(history.document)]
 const save=()=>{
  const document=history.document
  for(const item of document.bodies){
   expect(inspectNurbsBrep(item.brep!).topologyValid).toBe(true)
   const topology=item.brep!.topologyIds!
   expect(topology).toBeDefined()
   for(const kind of ['vertices','edges','faces','loops','shells','bodies'] as const){
    expect(topology[kind]).toHaveLength(item.brep![kind].length)
    expect(new Set(topology[kind]).size).toBe(topology[kind].length)
   }
   const display=tessellateNurbsBrep(item.brep!,2)
   expect(display.topologyFaceIds).toEqual(display.faceIds.map(i=>topology.faces[i]))
  }
  const serialized=stringifyMeshJson(document)
  expect(stringifyMeshJson(parseDirectDocument(serialized))).toBe(serialized)
  snapshots.push(serialized)
 }
 for(let cycle=0;cycle<4;cycle++){
  const previous=history.document
  const translated=transformSelection(previous,['part'],[1,0,0],[0,0,1],0,1)
  expect(translated.bodies[0].brep!.topologyIds).toEqual(previous.bodies[0].brep!.topologyIds)
  history.commit(translated);save()
  const pushed=history.document,top=solidTopology(pushed.bodies[0].mesh).faces.findIndex(f=>f.normal[2]>.99)
  pushed.bodies[0]=pushPullFace(pushed.bodies[0],top,1);history.commit(pushed);save()
  const split=history.document,pieces=splitSolid(split.bodies[0],[0,0,1],5)
  pieces[1].id='piece-'+cycle;split.bodies=pieces;history.commit(split);save()
  const merged=history.document
  merged.bodies=[{...body(booleanNurbsBrep(merged.bodies[0].brep!,merged.bodies[1].brep!,'union')),group:'assembly'}]
  history.commit(merged);save()
  const grouped=history.document;grouped.groups=[{name:'assembly',source:'revision '+cycle}];history.commit(grouped);save()
 }
 expect(snapshots).toHaveLength(21);expect(new Set(snapshots).size).toBe(21)
 for(let i=19;i>=0;i--)expect(stringifyMeshJson(history.undo())).toBe(snapshots[i])
 for(let i=1;i<=20;i++)expect(stringifyMeshJson(history.redo())).toBe(snapshots[i])
 const restored=new DirectHistory(parseDirectDocument(snapshots.at(-1)!))
 expect(stringifyMeshJson(restored.document)).toBe(snapshots.at(-1))
},20000)

it('preserves bracket, rational flange and hollow enclosure through 20 affine edits and every history snapshot',async()=>{
 const parts=await controlParts()
 const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 for(const [name,brep,volume] of parts){
  const history=new DirectHistory({version:1,sketches:[],bodies:[body(brep,name)]})
  const ids=history.document.bodies[0].brep!.topologyIds
  const snapshots=[stringifyMeshJson(history.document)]
  let accumulatedScale=1
  for(let edit=0;edit<20;edit++){
   const scale=edit%2===0?2:0.5
   const previous=stringifyMeshJson(history.document)
   const candidate=transformSelection(history.document,[name],[1,-2,3],[0,0,1],90,scale)
   // Preparing a result must leave the current history document untouched.
   expect(stringifyMeshJson(history.document)).toBe(previous)
   history.commit(candidate);accumulatedScale*=scale
   const result=history.document.bodies[0]
   expect(result.id).toBe(name)
   expect(result.brep!.topologyIds).toEqual(ids)
   expect(inspectNurbsBrep(result.brep!).topologyValid).toBe(true)
   const measured=analyzeNurbsBrep(result.brep!).signedVolumeMm3
   expect(Math.abs(measured-volume*accumulatedScale**3)).toBeLessThan(volume*accumulatedScale**3*1e-8)
   const serialized=stringifyMeshJson(history.document)
   expect(stringifyMeshJson(parseDirectDocument(serialized))).toBe(serialized)
   snapshots.push(serialized)
  }
  expect(new Set(snapshots).size).toBe(21)
  for(let i=19;i>=0;i--)expect(stringifyMeshJson(history.undo())).toBe(snapshots[i])
  for(let i=1;i<=20;i++)expect(stringifyMeshJson(history.redo())).toBe(snapshots[i])
  const restored=new DirectHistory(parseDirectDocument(snapshots[20]))
  expect(stringifyMeshJson(restored.document)).toBe(snapshots[20])
 }
},60000)

async function controlParts(){
 const {extrudeSketchProfile}=await import('../src/services/directExtrusion')
 const bracket=extrudeSketchProfile([{id:'bracket-profile',name:'Bracket',closed:true,points:[[0,0],[40,0],[40,5],[5,5],[5,30],[0,30]]}],20)
 const circle=(id:string,radius:number)=>({id,name:id,closed:true,points:[] as [number,number][],analytic:{kind:'circle' as const,center:[0,0] as [number,number],radius,start:0,sweep:360}})
 const flange=extrudeSketchProfile([circle('outer',20),circle('hole',5)],6)
 const enclosure=booleanNurbsBrep(createBrepBox([0,0,0],[40,30,20]),createBrepBox([2,2,2],[38,28,22]),'difference')
 return [['bracket',bracket,6500],['flange',flange,2250*Math.PI],['enclosure',enclosure,7152]] as const
}

it.each(['bracket','flange','enclosure'])('pushes the upper support of %s with preserved identity and analytic volume',async name=>{
 const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 const part=(await controlParts()).find(p=>p[0]===name)!
 const [,brep,volume]=part
 {
  const source=body(brep,name),before=stringifyMeshJson(source)
  const faces=solidTopology(source.mesh).faces
  const top=faces.map((f,i)=>({f,i})).filter(({f})=>f.normal[2]>.99).sort((a,b)=>b.f.center[2]-a.f.center[2])[0]?.i??-1
  expect(top).toBeGreaterThanOrEqual(0)
  const edited=pushPullFace(source,top,1)
  expect(edited.id).toBe(source.id)
  expect(stringifyMeshJson(source)).toBe(before)
  expect(inspectNurbsBrep(edited.brep!).topologyValid).toBe(true)
  const area=name==='bracket'?325:name==='flange'?375*Math.PI:264
  expect(analyzeNurbsBrep(edited.brep!).signedVolumeMm3).toBeCloseTo(volume+area,4)
 }
},30000)

it('matches native cap edits through the public WASM selection path',()=>{
 const fixtures=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/parts-history/cap-api-fixtures.json',import.meta.url),'utf8')).cases
 for(const fixture of fixtures){
  const request=fixture.request,source=request.body as DirectBody,before=stringifyMeshJson(source)
  const edited=pushPullFace(source,request.faces[0],request.amount)
  expect(edited.brep).toEqual(fixture.response.value.brep)
  expect(edited.id).toBe(source.id)
  expect(stringifyMeshJson(edited.mesh)).toBe(stringifyMeshJson(fixture.response.value.mesh))
  expect(stringifyMeshJson(source)).toBe(before)
  expect(()=>pushPullFace(source,request.faces[0],-20)).toThrow()
 }
})

it.each(['bracket','flange','enclosure'])('preserves 20 sequential cap edits and complete undo/redo for %s',async name=>{
 const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 const [,brep,volume]=(await controlParts()).find(p=>p[0]===name)!
 const area=name==='bracket'?325:name==='flange'?375*Math.PI:264
 const history=new DirectHistory({version:1,sketches:[],bodies:[body(brep,name)]})
 const snapshots=[stringifyMeshJson(history.document)]
 for(let edit=0;edit<20;edit++){
  const candidate=history.document,source=candidate.bodies[0]
  const top=solidTopology(source.mesh).faces.map((f,i)=>({f,i})).filter(({f})=>f.normal[2]>.99).sort((a,b)=>b.f.center[2]-a.f.center[2])[0].i
  candidate.bodies[0]=pushPullFace(source,top,0.25)
  expect(stringifyMeshJson(history.document)).toBe(snapshots[edit])
  history.commit(candidate)
  const result=history.document.bodies[0]
  expect(result.id).toBe(name)
  for(const kind of ['vertices','edges','loops','faces','shells','bodies'] as const){
   expect([...result.brep!.topologyIds![kind]].sort()).toEqual([...brep.topologyIds![kind]].sort())
  }
  // A vertex keeps its authored identity at its expected displaced location.
  const oldTop=Math.max(...source.brep!.vertices.map(v=>v.point[2]))
  for(let i=0;i<source.brep!.vertices.length;i++){
   const old=source.brep!.vertices[i].point
   const expected=[old[0],old[1],old[2]+(Math.abs(old[2]-oldTop)<1e-8?0.25:0)]
   const index=result.brep!.vertices.findIndex(v=>v.point.every((x,j)=>Math.abs(x-expected[j])<1e-8))
   expect(index).toBeGreaterThanOrEqual(0)
   expect(result.brep!.topologyIds!.vertices[index]).toBe(source.brep!.topologyIds!.vertices[i])
  }
  expect(inspectNurbsBrep(result.brep!).topologyValid).toBe(true)
  expect(analyzeNurbsBrep(result.brep!).signedVolumeMm3).toBeCloseTo(volume+area*(edit+1)*0.25,4)
  const serialized=stringifyMeshJson(history.document)
  expect(stringifyMeshJson(parseDirectDocument(serialized))).toBe(serialized)
  snapshots.push(serialized)
 }
 expect(new Set(snapshots).size).toBe(21)
 for(let i=19;i>=0;i--)expect(stringifyMeshJson(history.undo())).toBe(snapshots[i])
 for(let i=1;i<=20;i++)expect(stringifyMeshJson(history.redo())).toBe(snapshots[i])
 expect(stringifyMeshJson(new DirectHistory(parseDirectDocument(snapshots[20])).document)).toBe(snapshots[20])
},60000)

it.each(['bracket','flange','enclosure'])('preserves %s through 20 mixed cap, rotation, translation and scale edits',async name=>{
 const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 const [,brep,initialVolume]=(await controlParts()).find(p=>p[0]===name)!
 let volume=initialVolume,area=name==='bracket'?325:name==='flange'?375*Math.PI:264
 const history=new DirectHistory({version:1,sketches:[],bodies:[body(brep,name)]})
 const snapshots=[stringifyMeshJson(history.document)]
 const check=()=>{
  const result=history.document.bodies[0]
  expect(result.id).toBe(name)
  expect(inspectNurbsBrep(result.brep!).topologyValid).toBe(true)
  expect(analyzeNurbsBrep(result.brep!).signedVolumeMm3).toBeCloseTo(volume,4)
  for(const kind of ['vertices','edges','loops','faces','shells','bodies'] as const)
   expect([...result.brep!.topologyIds![kind]].sort()).toEqual([...brep.topologyIds![kind]].sort())
  const serialized=stringifyMeshJson(history.document)
  expect(stringifyMeshJson(parseDirectDocument(serialized))).toBe(serialized)
  snapshots.push(serialized)
 }
 for(let cycle=0;cycle<4;cycle++){
  const candidate=history.document,source=candidate.bodies[0]
  const top=solidTopology(source.mesh).faces.map((f,i)=>({f,i})).filter(({f})=>f.normal[2]>.99).sort((a,b)=>b.f.center[2]-a.f.center[2])[0].i
  const before=stringifyMeshJson(history.document)
  // A discarded preview and a refused displacement must not enter history.
  pushPullFace(source,top,0.125)
  expect(()=>pushPullFace(source,top,-10000)).toThrow()
  expect(stringifyMeshJson(history.document)).toBe(before)
  candidate.bodies[0]=pushPullFace(source,top,0.5)
  volume+=area*0.5;history.commit(candidate);check()
  history.commit(transformSelection(history.document,[name],[0,0,0],[0,0,1],90,1));check()
  history.commit(transformSelection(history.document,[name],[3,-2,1],[0,0,1],0,1));check()
  history.commit(transformSelection(history.document,[name],[0,0,0],[0,0,1],0,2));volume*=8;area*=4;check()
  history.commit(transformSelection(history.document,[name],[0,0,0],[0,0,1],0,0.5));volume/=8;area/=4;check()
 }
 expect(snapshots).toHaveLength(21)
 for(let i=19;i>=0;i--)expect(stringifyMeshJson(history.undo())).toBe(snapshots[i])
 for(let i=1;i<=20;i++)expect(stringifyMeshJson(history.redo())).toBe(snapshots[i])
 expect(stringifyMeshJson(new DirectHistory(parseDirectDocument(snapshots[20])).document)).toBe(snapshots[20])
},60000)
