import {expect,it} from 'vitest'
import {extrudePolygonProfile} from '../src/services/geometry/polygon'
import {inspectStructuralSections} from '../src/services/structuralSections'
import {mainSolidResult,isBoundaryConnectivity} from '../src/services/mainSolidProtocol'

const outer=[[0,0],[10,0],[10,10],[0,10]]
const build=(holes:number[][][]=[])=>extrudePolygonProfile({outer,holes},[0,0,10])
it('uses actual final boundaries instead of identical bounding boxes through WASM',()=>{
  const solid=inspectStructuralSections(build(),'z',[5])
  const hollow=inspectStructuralSections(build([[[2,2],[8,2],[8,8],[2,8]]]),'z',[5])
  expect(solid.sections[0].properties?.areaMm2).toBeCloseTo(100,9)
  expect(hollow.sections[0].properties?.areaMm2).toBeCloseTo(64,9)
  expect(hollow.sections[0].properties?.iuuMm4).toBeCloseTo((10000-1296)/12,8)
  expect(hollow.sourceMesh.indices.length/3).toBe(hollow.triangleCount)
  const expected={kind:'structuralSections' as const,axis:'z' as const,stations:[5]}
  expect(mainSolidResult(expected,hollow)).toBe(true)
  expect(mainSolidResult({...expected,stations:[6]},hollow)).toBe(false)
  expect(mainSolidResult({...expected,axis:'x'},hollow)).toBe(false)
  expect(mainSolidResult(expected,{...hollow,sections:[{...hollow.sections[0],properties:{}}]})).toBe(false)
})
it('preserves separate material regions and empty sections without zero stiffness claims',()=>{
  const report=inspectStructuralSections(build([[[2,2],[8,2],[8,8],[2,8]]]),'x',[-1,5,10])
  expect(report.sections.map(s=>s.material)).toEqual([false,true,false])
  expect(report.sections[1].properties?.areaMm2).toBeCloseTo(40,9)
  expect(report.sections[0].properties).toBeNull()
  expect(report.planeAxes).toEqual([1,2])
})
it('refuses invalid geometry and station order',()=>{
  const mesh=build()
  expect(()=>inspectStructuralSections({...mesh,indices:mesh.indices.slice(3)},'z',[5])).toThrow()
  for(const stations of [[],[5,5],[6,5],Array(65).fill(1)])expect(()=>inspectStructuralSections(mesh,'z',stations)).toThrow()
})

it('exports bounded edge-component provenance and rejects damaged worker reports',()=>{
  const report=inspectStructuralSections(build(),'z',[5])
  const c=report.connectivity
  expect(c.components).toHaveLength(1)
  expect(c.components[0].signedVolumeMm3).toBeCloseTo(1000)
  expect(c.components[0].sourceTriangles).toHaveLength(report.triangleCount)
  expect(c.sharedVertices).toEqual([])
  expect(c.materialConnectivity).toBe('classified-at-tolerance')
  const valid=(v:unknown)=>isBoundaryConnectivity(v,report.triangleCount,report.sourceMesh.positions.length/3)
  expect(valid(c)).toBe(true)
  expect(valid(undefined)).toBe(false)
  expect(valid({...c,components:[{...c.components[0],sourceTriangles:[0,0]}]})).toBe(false)
  expect(valid({...c,components:[{...c.components[0],boundsMm:[[1,0,0],[0,0,0]]}]})).toBe(false)
  expect(valid({...c,sharedVertices:[{sourceVertex:0,components:[0,0]}]})).toBe(false)
  expect(mainSolidResult({kind:'structuralSections',axis:'z',stations:[5]},{...report,connectivity:null})).toBe(false)
})

it('keeps separate solids separate through the WASM transport',()=>{
  const a=build(),b=build(),offset=a.positions.length/3
  const mesh={positions:[...a.positions,...b.positions.map((p,i)=>p+(i%3===0?20:0))],indices:[...a.indices,...b.indices.map(i=>i+offset)]}
  const report=inspectStructuralSections(mesh,'z',[5])
  expect(report.connectivity.components).toHaveLength(2)
  expect(report.connectivity.components.map(c=>c.signedVolumeMm3)).toEqual([1000,1000])
  expect(report.connectivity.sharedVertices).toEqual([])
  expect(report.sections[0].properties?.areaMm2).toBeCloseTo(200)
  expect(mainSolidResult({kind:'structuralSections',axis:'z',stations:[5]},report)).toBe(true)
})

it('classifies closed cavities separately from material islands and refuses contacts',()=>{
  function combine(innerScale:number,offset:number,reverse:boolean){
    const a=build(),b=build(),n=a.positions.length/3
    const indices=[...b.indices]
    if(reverse)for(let i=0;i<indices.length;i+=3)[indices[i+1],indices[i+2]]=[indices[i+2],indices[i+1]]
    return {positions:[...a.positions,...b.positions.map(p=>p*innerScale+offset)],indices:[...a.indices,...indices.map(i=>i+n)]}
  }
  const report=inspectStructuralSections(combine(.6,2,true),'z',[5])
  const expected={kind:'structuralSections' as const,axis:'z' as const,stations:[5]}
  expect(report.materialAudit.status).toBe('classified')
  if(report.materialAudit.status!=='classified')throw Error('Missing material classification')
  expect(report.materialAudit.materialRegions).toBe(1)
  expect(report.materialAudit.shells.map(s=>[s.kind,s.parent])).toEqual([['material',null],['cavity',0]])
  expect(mainSolidResult(expected,report)).toBe(true)
  expect(mainSolidResult(expected,{...report,materialAudit:undefined})).toBe(false)
  expect(mainSolidResult(expected,{...report,materialAudit:{...report.materialAudit,materialRegions:2}})).toBe(false)
  expect(mainSolidResult(expected,{...report,materialAudit:{...report.materialAudit,shells:report.materialAudit.shells.map(s=>({...s,parent:1}))}})).toBe(false)
  const touching=inspectStructuralSections(combine(1,10,false),'z',[-1])
  expect(touching.materialAudit.status).toBe('unresolved')
  expect(touching.connectivity.materialConnectivity).toBe('not-established')
  expect(mainSolidResult({...expected,stations:[-1]},touching)).toBe(true)
})
