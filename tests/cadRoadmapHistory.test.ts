import {readFileSync,mkdirSync,writeFileSync,existsSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {expect,it} from 'vitest'
import {DirectHistory,parseDirectDocument,type DirectBody} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep,booleanNurbsBrep,inspectNurbsBrep,type NurbsBrep} from '../src/services/geometry/brep'
import {transformSelection,pushPullFace,solidTopology,splitSolid} from '../src/services/directSolidTools'
import {stringifyMeshJson} from '../src/services/meshJson'
// Evidence comes from the final body of each exercised history, not a reconstructed fixture.
async function exportMixedHistoryEvidence(body:DirectBody,volume:number,bounds:number[][]){
 const directory=process.env.CAD_MIXED20_STEP_OUTPUT
 if(!directory)return
 const {exportSolidStepCurrent}=await import('../src/services/solidStepExchange')
 const {importDirectStepV9}=await import('../src/services/cadNurbsStep')
 mkdirSync(directory,{recursive:true})
 const manifestPath=resolve(directory,'manifest.json')
 const manifest=existsSync(manifestPath)?JSON.parse(readFileSync(manifestPath,'utf8')):{schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[]}
 const first=await exportSolidStepCurrent(body)
 const second=await exportSolidStepCurrent({...body,brep:importDirectStepV9(first).model})
 manifest.parts=manifest.parts.filter((p:{name:string})=>!p.name.startsWith(body.id+'-mixed20-'))
 for(const [cycle,text] of [first,second].entries()){
  const name=body.id+'-mixed20-'+cycle,file=name+'.step'
  writeFileSync(resolve(directory,file),text)
  manifest.parts.push({name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:volume,boundsMm:bounds}})
 }
 writeFileSync(resolve(directory,body.id+'-final.json'),stringifyMeshJson({version:1,sketches:[],bodies:[body]}))
 writeFileSync(manifestPath,JSON.stringify(manifest,null,2)+'\n')
}
// Capture actual operands and checked history for the UI's mixed-operation replay.
function exportMixedUiTool(name:string,cycle:number,tool:NurbsBrep){
 const root=process.env.CAD_MIXED20_UI_OUTPUT;if(!root)return
 const directory=resolve(root,name);mkdirSync(directory,{recursive:true})
 const mesh=tessellateNurbsBrep(tool,8)
 writeFileSync(resolve(directory,`tool-${cycle}.json`),stringifyMeshJson({id:`tool-${cycle}`,name:`Tool ${cycle}`,brep:tool,mesh}))
}
function exportMixedUiHistory(name:string,snapshots:string[]){
 const root=process.env.CAD_MIXED20_UI_OUTPUT;if(!root)return
 const directory=resolve(root,name);mkdirSync(directory,{recursive:true})
 const tools=Array.from({length:6},(_,i)=>JSON.parse(readFileSync(resolve(directory,`tool-${i}.json`),'utf8')))
 const initial=JSON.parse(snapshots[0]);initial.bodies.push(...tools)
 writeFileSync(resolve(directory,'initial.json'),JSON.stringify(initial))
 writeFileSync(resolve(directory,'expected.json'),`[${snapshots.join(',')}]`)
}
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

it.each([2,-2])('preserves dimensions through union, cap push %s mm, exact fillet, STEP and history',async(distance)=>{
 const {solidExactEdgeFeature}=await import('../src/services/solidExactEdgeFeature')
 const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 const {exportSolidStepCurrent}=await import('../src/services/solidStepExchange')
 const {importDirectStepV9}=await import('../src/services/cadNurbsStep')
 const a=createBrepBox([0,0,0],[12,10,10]),b=createBrepBox([8,0,0],[20,10,10])
 const history=new DirectHistory({version:1,sketches:[],bodies:[body(a,'a'),body(b,'b')]})
 const snapshots=[stringifyMeshJson(history.document)]
 const commit=(part:DirectBody)=>{history.commit({version:1,sketches:[],bodies:[part]});snapshots.push(stringifyMeshJson(history.document))}
 const joined=body(booleanNurbsBrep(a,b,'union'));commit(joined)
 const top=solidTopology(joined.mesh).faces.map((face,index)=>({face,index})).filter(({face})=>face.normal[2]>.99).sort((a,b)=>b.face.center[2]-a.face.center[2])[0].index
 const pushed=pushPullFace(joined,top,distance);commit(pushed)
 const edges=pushed.brep!.edges.flatMap((edge,index)=>{
  const [a,b]=edge.vertices.map(i=>pushed.brep!.vertices[i].point)
  return a[0]===b[0]&&a[1]===b[1]?[index]:[]
 })
 expect(edges).toHaveLength(4)
 const rounded=solidExactEdgeFeature(pushed,edges,1,'fillet','brep').body;commit(rounded)
 const imported=importDirectStepV9(await exportSolidStepCurrent(rounded)).model
 expect(inspectNurbsBrep(imported).topologyValid).toBe(true)
 expect(analyzeNurbsBrep(imported).signedVolumeMm3).toBeCloseTo((200-4+Math.PI)*(10+distance),5)
 for(let axis=0;axis<3;axis++){
  expect(Math.min(...imported.vertices.map(v=>v.point[axis]))).toBeCloseTo(0,6)
  expect(Math.max(...imported.vertices.map(v=>v.point[axis]))).toBeCloseTo([20,10,10+distance][axis],6)
 }
 for(let i=snapshots.length-2;i>=0;i--)expect(stringifyMeshJson(history.undo())).toBe(snapshots[i])
 for(let i=1;i<snapshots.length;i++)expect(stringifyMeshJson(history.redo())).toBe(snapshots[i])
 expect(parseDirectDocument(stringifyMeshJson(history.document)).bodies[0].brep!.topologyIds).toEqual(rounded.brep.topologyIds)
})

it('preserves the bracket through 20 Boolean, face, transform and exact-fillet edits',async()=>{
 const {cadRoadmapParts}=await import('../benchmarks/cad-roadmap-fixtures')
 const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 const {solidExactEdgeFeature}=await import('../src/services/solidExactEdgeFeature')
 const initial=cadRoadmapParts().find(p=>p.body.id==='bracket')!.body
 const history=new DirectHistory({version:1,sketches:[],bodies:[initial]})
 const snapshots=[stringifyMeshJson(history.document)]
 let height=20,length=40,offset=0,area=325
 const save=(candidate:DirectBody,volume:number)=>{
  const ids=candidate.brep!.topologyIds!
  expect(candidate.id).toBe('bracket')
  expect(inspectNurbsBrep(candidate.brep!).topologyValid).toBe(true)
  expect(analyzeNurbsBrep(candidate.brep!).signedVolumeMm3).toBeCloseTo(volume,4)
  for(let axis=0;axis<3;axis++){
   expect(Math.min(...candidate.brep!.vertices.map(v=>v.point[axis]))).toBeCloseTo([offset,0,0][axis],6)
   expect(Math.max(...candidate.brep!.vertices.map(v=>v.point[axis]))).toBeCloseTo([offset+length,30,height][axis],6)
  }
  for(const kind of ['vertices','edges','faces','loops','shells','bodies'] as const){
   expect(ids[kind]).toHaveLength(candidate.brep![kind].length)
   expect(new Set(ids[kind]).size).toBe(ids[kind].length)
  }
  history.commit({...history.document,bodies:[candidate]})
  const serialized=stringifyMeshJson(history.document)
  expect(stringifyMeshJson(parseDirectDocument(serialized))).toBe(serialized)
  snapshots.push(serialized)
 }
 for(let cycle=0;cycle<6;cycle++){
  const source=history.document.bodies[0],before=stringifyMeshJson(source)
  const top=solidTopology(source.mesh).faces.findIndex(f=>f.normal[2]>.99)
  const pushed=pushPullFace(source,top,1);height++
  expect(stringifyMeshJson(source)).toBe(before);save(pushed,area*height)
  const current=history.document.bodies[0],tool=createBrepBox([offset+length-1,0,0],[offset+length+1,5,height])
  exportMixedUiTool('bracket',cycle,tool)
  const merged=booleanNurbsBrep(current.brep!,tool,'union');length++;area+=5
  save({...current,brep:merged,mesh:tessellateNurbsBrep(merged,2)},area*height)
  const moved=transformSelection(history.document,['bracket'],[1,0,0],[0,0,1],0,1).bodies[0];offset++
  save(moved,area*height)
 }
 const source=history.document.bodies[0],before=stringifyMeshJson(source)
 const edge=source.brep!.edges.findIndex(e=>e.vertices.every(i=>{const p=source.brep!.vertices[i].point;return Math.abs(p[0]-offset)<1e-8&&Math.abs(p[1])<1e-8}))
 expect(edge).toBeGreaterThanOrEqual(0)
 const rounded=solidExactEdgeFeature(source,[edge],1,'fillet','brep').body
 expect(stringifyMeshJson(source)).toBe(before)
 const volume=(area-(1-Math.PI/4))*height;save(rounded,volume)
 const moved=transformSelection(history.document,['bracket'],[1,0,0],[0,0,1],0,1).bodies[0];offset++
 save(moved,volume)
 const {exportSolidStepCurrent}=await import('../src/services/solidStepExchange')
 const {importDirectStepV9}=await import('../src/services/cadNurbsStep')
 exportMixedUiHistory(initial.id,snapshots)
 await exportMixedHistoryEvidence(moved,volume,[[offset,0,0],[offset+length,30,height]])
 const imported=importDirectStepV9(await exportSolidStepCurrent(moved)).model
 expect(inspectNurbsBrep(imported).topologyValid).toBe(true)
 expect(analyzeNurbsBrep(imported).signedVolumeMm3).toBeCloseTo(volume,4)
 const vertices=moved.brep!.vertices
 for(let axis=0;axis<3;axis++){
  expect(Math.min(...vertices.map(v=>v.point[axis]))).toBeCloseTo([offset,0,0][axis],6)
  expect(Math.max(...vertices.map(v=>v.point[axis]))).toBeCloseTo([offset+length,30,height][axis],6)
 }
 expect(snapshots).toHaveLength(21);expect(new Set(snapshots).size).toBe(21)
 for(let i=19;i>=0;i--)expect(stringifyMeshJson(history.undo())).toBe(snapshots[i])
 for(let i=1;i<=20;i++)expect(stringifyMeshJson(history.redo())).toBe(snapshots[i])
 expect(stringifyMeshJson(new DirectHistory(parseDirectDocument(snapshots[20])).document)).toBe(snapshots[20])
},60000)

it('preserves the flange through 20 mixed face, hole-Boolean, transform and circular-fillet edits',async()=>{
 const {cadRoadmapParts}=await import('../benchmarks/cad-roadmap-fixtures')
 const {analyzeNurbsBrep,createBrepCylinder,transformNurbsBrep}=await import('../src/services/geometry/brep')
 const {solidExactEdgeFeature}=await import('../src/services/solidExactEdgeFeature')
 const initial=cadRoadmapParts().find(p=>p.body.id==='flange')!.body
 const history=new DirectHistory({version:1,sketches:[],bodies:[initial]})
 const snapshots=[stringifyMeshJson(history.document)]
 let height=6,inner=5,offset=0
 const save=(candidate:DirectBody,volume:number)=>{
  expect(candidate.id).toBe('flange')
  expect(inspectNurbsBrep(candidate.brep!).topologyValid).toBe(true)
  expect(analyzeNurbsBrep(candidate.brep!).signedVolumeMm3).toBeCloseTo(volume,4)
  for(let axis=0;axis<3;axis++){
   expect(Math.min(...candidate.brep!.vertices.map(v=>v.point[axis]))).toBeCloseTo([offset-20,-20,0][axis],6)
   expect(Math.max(...candidate.brep!.vertices.map(v=>v.point[axis]))).toBeCloseTo([offset+20,20,height][axis],6)
  }
  const ids=candidate.brep!.topologyIds!
  for(const kind of ['vertices','edges','faces','loops','shells','bodies'] as const){
   expect(ids[kind]).toHaveLength(candidate.brep![kind].length)
   expect(new Set(ids[kind]).size).toBe(ids[kind].length)
  }
  history.commit({...history.document,bodies:[candidate]})
  const snapshot=stringifyMeshJson(history.document)
  expect(stringifyMeshJson(parseDirectDocument(snapshot))).toBe(snapshot);snapshots.push(snapshot)
 }
 for(let cycle=0;cycle<6;cycle++){
  const source=history.document.bodies[0],before=stringifyMeshJson(source)
  const top=solidTopology(source.mesh).faces.findIndex(f=>f.normal[2]>.99)
  const pushed=pushPullFace(source,top,1);height++
  expect(stringifyMeshJson(source)).toBe(before);save(pushed,Math.PI*(400-inner**2)*height)
  const current=history.document.bodies[0];inner+=.25
  const tool=transformNurbsBrep(createBrepCylinder(inner,height),[[1,0,0,offset],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
  exportMixedUiTool(initial.id,cycle,tool)
  const cut=booleanNurbsBrep(current.brep!,tool,'difference')
  save({...current,brep:cut,mesh:tessellateNurbsBrep(cut,8)},Math.PI*(400-inner**2)*height)
  const moved=transformSelection(history.document,['flange'],[1,0,0],[0,0,1],0,1).bodies[0];offset++
  save(moved,Math.PI*(400-inner**2)*height)
 }
 const source=history.document.bodies[0],before=stringifyMeshJson(source)
 const edges=source.brep!.edges.flatMap((e,i)=>e.curve.degree===2&&e.vertices.every(v=>{
  const p=source.brep!.vertices[v].point;return Math.abs(p[2]-height)<1e-8&&Math.abs(Math.hypot(p[0]-offset,p[1])-20)<1e-8
 })?[i]:[])
 expect(edges).toHaveLength(4)
 const rounded=solidExactEdgeFeature(source,edges,1,'fillet','brep').body
 expect(stringifyMeshJson(source)).toBe(before)
 const volume=Math.PI*(400-inner**2)*height-2*Math.PI*(19*(1-Math.PI/4)+1/6)
 save(rounded,volume)
 const moved=transformSelection(history.document,['flange'],[1,0,0],[0,0,1],0,1).bodies[0];offset++
 save(moved,volume)
 expect(snapshots).toHaveLength(21);expect(new Set(snapshots).size).toBe(21)
 const {exportSolidStepCurrent}=await import('../src/services/solidStepExchange')
 const {importDirectStepV9}=await import('../src/services/cadNurbsStep')
 exportMixedUiHistory(initial.id,snapshots)
 await exportMixedHistoryEvidence(moved,volume,[[offset-20,-20,0],[offset+20,20,height]])
 const imported=importDirectStepV9(await exportSolidStepCurrent(moved)).model
 expect(inspectNurbsBrep(imported).topologyValid).toBe(true)
 expect(analyzeNurbsBrep(imported).signedVolumeMm3).toBeCloseTo(volume,4)
 for(let i=19;i>=0;i--)expect(stringifyMeshJson(history.undo())).toBe(snapshots[i])
 for(let i=1;i<=20;i++)expect(stringifyMeshJson(history.redo())).toBe(snapshots[i])
 expect(stringifyMeshJson(new DirectHistory(parseDirectDocument(snapshots[20])).document)).toBe(snapshots[20])
},60000)

it('preserves the enclosure through 20 mixed face, hole-Boolean, transform and layered-fillet edits',async()=>{
 const {cadRoadmapParts}=await import('../benchmarks/cad-roadmap-fixtures')
 const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 const {solidExactEdgeFeature}=await import('../src/services/solidExactEdgeFeature')
 const initial=cadRoadmapParts().find(p=>p.body.id==='enclosure')!.body
 const history=new DirectHistory({version:1,sketches:[],bodies:[initial]})
 const snapshots=[stringifyMeshJson(history.document)]
 let height=20,inset=2,offset=0
 const volumeNow=()=>1200*height-(40-2*inset)*26*(height-2)
 const save=(candidate:DirectBody,volume:number)=>{
  expect(candidate.id).toBe('enclosure')
  expect(inspectNurbsBrep(candidate.brep!).topologyValid).toBe(true)
  expect(analyzeNurbsBrep(candidate.brep!).signedVolumeMm3).toBeCloseTo(volume,4)
  for(let axis=0;axis<3;axis++){
   expect(Math.min(...candidate.brep!.vertices.map(v=>v.point[axis]))).toBeCloseTo([offset,0,0][axis],6)
   expect(Math.max(...candidate.brep!.vertices.map(v=>v.point[axis]))).toBeCloseTo([offset+40,30,height][axis],6)
  }
  const ids=candidate.brep!.topologyIds!
  for(const kind of ['vertices','edges','faces','loops','shells','bodies'] as const){
   expect(ids[kind]).toHaveLength(candidate.brep![kind].length)
   expect(new Set(ids[kind]).size).toBe(ids[kind].length)
  }
  history.commit({...history.document,bodies:[candidate]})
  const snapshot=stringifyMeshJson(history.document)
  expect(stringifyMeshJson(parseDirectDocument(snapshot))).toBe(snapshot);snapshots.push(snapshot)
 }
 for(let cycle=0;cycle<6;cycle++){
  const source=history.document.bodies[0],before=stringifyMeshJson(source)
  const top=solidTopology(source.mesh).faces.findIndex(f=>f.normal[2]>.99)
  const pushed=pushPullFace(source,top,1);height++
  expect(stringifyMeshJson(source)).toBe(before);save(pushed,volumeNow())
  const current=history.document.bodies[0];inset-=.1
  const tool=createBrepBox([offset+inset,2,2],[offset+40-inset,28,height+2])
  exportMixedUiTool(initial.id,cycle,tool)
  const cut=booleanNurbsBrep(current.brep!,tool,'difference')
  save({...current,brep:cut,mesh:tessellateNurbsBrep(cut,8)},volumeNow())
  const moved=transformSelection(history.document,['enclosure'],[1,0,0],[0,0,1],0,1).bodies[0];offset++
  save(moved,volumeNow())
 }
 const source=history.document.bodies[0],before=stringifyMeshJson(source)
 const edges=source.brep!.edges.flatMap((e,i)=>e.vertices.every(v=>{
  const p=source.brep!.vertices[v].point;return Math.abs(p[0]-offset)<1e-8&&Math.abs(p[1])<1e-8
 })?[i]:[])
 expect(edges.length).toBeGreaterThan(1)
 const rounded=solidExactEdgeFeature(source,edges,1,'fillet','brep').body
 expect(stringifyMeshJson(source)).toBe(before)
 const volume=volumeNow()-(1-Math.PI/4)*height
 save(rounded,volume)
 const moved=transformSelection(history.document,['enclosure'],[1,0,0],[0,0,1],0,1).bodies[0];offset++
 save(moved,volume)
 expect(snapshots).toHaveLength(21);expect(new Set(snapshots).size).toBe(21)
 const {exportSolidStepCurrent}=await import('../src/services/solidStepExchange')
 const {importDirectStepV9}=await import('../src/services/cadNurbsStep')
 exportMixedUiHistory(initial.id,snapshots)
 await exportMixedHistoryEvidence(moved,volume,[[offset,0,0],[offset+40,30,height]])
 const imported=importDirectStepV9(await exportSolidStepCurrent(moved)).model
 expect(inspectNurbsBrep(imported).topologyValid).toBe(true)
 expect(analyzeNurbsBrep(imported).signedVolumeMm3).toBeCloseTo(volume,4)
 for(let i=19;i>=0;i--)expect(stringifyMeshJson(history.undo())).toBe(snapshots[i])
 for(let i=1;i<=20;i++)expect(stringifyMeshJson(history.redo())).toBe(snapshots[i])
 expect(stringifyMeshJson(new DirectHistory(parseDirectDocument(snapshots[20])).document)).toBe(snapshots[20])
},60000)
