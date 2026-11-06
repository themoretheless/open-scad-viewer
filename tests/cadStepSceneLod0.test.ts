import {beforeEach,describe,expect,it,vi} from 'vitest'
import type {DirectStepLod0Import,DirectStepSceneImport} from '../src/services/cadNurbsStep'
import {parseDirectDocument} from '../src/services/directModeling'
import {stringifyMeshJson} from '../src/services/meshJson'

const {bridge,tessellate}=vi.hoisted(()=>({
  bridge:vi.fn<(op:string,args:{text:string})=>unknown>(),
  tessellate:vi.fn(()=>{throw Error('exact tessellation must not be used for mixed LOD-0')}),
}))
vi.mock('../src/services/geometry/kernel',()=>({callGeometryRust:bridge}))
vi.mock('../src/services/geometry/brep',async importOriginal=>({...await importOriginal<typeof import('../src/services/geometry/brep')>(),tessellateNurbsBrep:tessellate}))

import {importDirectStepLod0,importDirectStepScene} from '../src/services/cadNurbsStep'
import {importStepForWorkbench,inspectStepRoute} from '../src/services/cadStepRouting'
import {stepLod0Cache} from '../src/services/stepLod0Cache'

const model={bodies:[{outerShell:0,innerShells:[]}],vertices:[],edges:[],loops:[],faces:[],shells:[],topologyIds:{bodies:['body-1']}}
const document={source:'original',graphIdentity:'graph',definitionIdentities:['definition'],occurrenceIdentities:['occurrence'],productHierarchy:[],operatorIdentities:[],metadataLoss:['v10-loss']}
const scene:DirectStepSceneImport={
  model:model as never,
  certificate:{capability:'step-interchange/9',complete:true,notes:[]},
  report:{identity:{preserved:true,source:'test',preservedCount:1,createdCount:0,lostCount:0},ignoredEntities:[],instanceCount:1,reachableCount:1,metadataLoss:['scene-loss']},
  sceneTree:{kind:'stepScene',products:[{entity:42,id:'part-42',name:'Корпус δ',description:'',shapeName:'Основная форма',appliedNames:[],material:'Сталь 09Г2С',densityKgPerM3:7850,
    validation:{volumeMm3:24,surfaceAreaMm2:52},validationDiagnostics:[],display:{rgba:[0,0,255,255],layers:['Product layer']},
    bodies:[{bodyIndex:0,topologyBodyId:'body-1',display:{rgba:[0,255,0,255],layers:['Body layer']},faces:[{faceIndex:0,topologyFaceId:'face-1',display:{rgba:[255,0,0,255],layers:['Face layer']}}]}]}],
    unsupported:{presentation:[],xde:[]}},
  display:{products:[{productEntity:42,rgba:[0,0,255,255],layers:['Product layer']}],
    bodies:[{bodyIndex:0,topologyBodyId:'body-1',productEntity:42,rgba:[0,255,0,255],layers:['Body layer']}],
    faces:[{faceIndex:0,topologyFaceId:'face-1',bodyIndex:0,productEntity:42,rgba:[255,0,0,255],layers:['Face layer']}]},
}
const lod0:DirectStepLod0Import={
  source:'step-ap242-tessellated',cacheKey:'a'.repeat(64),cacheReady:true,requiresRetessellation:false,exactGeometryAuthoritative:true,exactBrepEntityIds:[4],
  diagnostics:[{code:'drift-not-assessed',severity:'warning',entityId:3,message:'drift unavailable'}],schemaDiagnostics:[],
  lod0Shells:[{shellEntityId:3,name:'shell',source:'step-ap242-tessellated',cacheKey:'b'.repeat(64),mesh:{positions:[0,0,0,2,0,0,0,3,0],indices:[0,1,2]},triangleCount:1,vertexCount:3,
    bounds:{min:[0,0,0],max:[2,3,0]},exactBrepEntityIds:[4],declaredTolerance:null,drift:{status:'not-assessed',hausdorffCertified:false},diagnostics:[]}],
}
const mixed='ISO-10303-21;DATA;#1=ADVANCED_BREP_SHAPE_REPRESENTATION();#2=MANIFOLD_SOLID_BREP();#3=TESSELLATED_SHELL();ENDSEC;END-ISO-10303-21;'
const pureTessellated=`ISO-10303-21;
HEADER;FILE_DESCRIPTION(('tessellated'),'2;1');FILE_NAME('t.step','',(''),(''),'','','');FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));ENDSEC;
DATA;#1=CARTESIAN_POINT_LIST_3D('',((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)));
#2=TRIANGULATED_FACE_SET('',#1,3,$,.F.,((1,2,3)),$);
#3=TESSELLATED_SHAPE_REPRESENTATION('',(#2),$);ENDSEC;END-ISO-10303-21;`

const values=new Map<string,string>()
Object.defineProperty(globalThis,'localStorage',{configurable:true,value:{
  getItem:(key:string)=>values.get(key)??null,setItem:(key:string,value:string)=>{values.set(key,value)},removeItem:(key:string)=>{values.delete(key)},
  key:(index:number)=>[...values.keys()][index]??null,get length(){return values.size},
}})

beforeEach(()=>{
  bridge.mockReset();tessellate.mockClear();stepLod0Cache.clear()
  bridge.mockImplementation(op=>{
    if(op==='brep_nurbs_import_step_v10')return {model,document,certificate:{capability:'step-interchange/10',complete:true,notes:[]}}
    if(op==='brep_nurbs_import_step_scene')return scene
    if(op==='brep_nurbs_import_step_lod0')return lod0
    throw Error(`unexpected op ${op}`)
  })
})

describe('STEP scene and baked LOD-0 frontend integration',()=>{
  it('exposes typed synchronous scene and LOD-0 wrappers',()=>{
    expect(importDirectStepScene('scene')).toBe(scene)
    expect(importDirectStepLod0('lod')).toBe(lod0)
    expect(bridge.mock.calls.map(call=>call[0])).toEqual(['brep_nurbs_import_step_scene','brep_nurbs_import_step_lod0'])
  })

  it('keeps mixed exact+tessellated AP242 retained and uses cached LOD-0 directly',()=>{
    const first=importStepForWorkbench(mixed)
    expect(inspectStepRoute(mixed)).toBe('retained-ap242-brep')
    expect(first.report.retained).toBe(true)
    expect(first.retainedModel).toBe(model)
    expect(first.bodies[0].brep).toBe(model)
    expect(Array.from(first.bodies[0].mesh.positions)).toEqual(lod0.lod0Shells[0].mesh.positions)
    expect(first.bodies[0]).toMatchObject({name:'Корпус δ',material:{name:'Сталь 09Г2С',color:'#00ff00'}})
    expect(first.report.display?.faces[0]?.rgba).toEqual([255,0,0,255])
    expect(first.report.metadataLoss).toEqual(expect.arrayContaining(['v10-loss','scene-loss']))
    expect(first.report.metadataLoss.some(item=>item.includes('drift-not-assessed'))).toBe(true)
    expect(tessellate).not.toHaveBeenCalled()

    const second=importStepForWorkbench(mixed)
    expect(second.lod0?.lod0Shells[0]).toBe(first.lod0?.lod0Shells[0])
    expect(bridge.mock.calls.filter(call=>call[0]==='brep_nurbs_import_step_lod0')).toHaveLength(1)
    stepLod0Cache.reload()
    expect(stepLod0Cache.get('b'.repeat(64))).toBeDefined()
    expect(stepLod0Cache.getDocument(mixed)).toBeDefined()
    expect(importStepForWorkbench(mixed).lod0?.cacheKey).toBe(lod0.cacheKey)
    expect(bridge.mock.calls.filter(call=>call[0]==='brep_nurbs_import_step_lod0')).toHaveLength(1)
  })

  it('keeps the pure tessellated route unchanged',()=>{
    const routed=importStepForWorkbench(pureTessellated)
    expect(routed.report).toMatchObject({route:'tessellated-shape',retained:false})
    expect(routed.retainedModel).toBeUndefined()
    expect(routed.lod0).toBeUndefined()
    expect(Array.from(routed.bodies[0].mesh.indices)).toEqual([0,1,2])
    expect(bridge).not.toHaveBeenCalled()
  })

  it('persists sceneTree/display/XDE through DirectDocument validation and keeps old reports compatible',()=>{
    const routed=importStepForWorkbench(mixed)
    const step=routed.report
    const mesh={positions:[0,0,0,2,0,0,0,3,0],indices:[0,1,2]}
    const parsed=parseDirectDocument(stringifyMeshJson({version:1,sketches:[],bodies:[{...routed.bodies[0],brep:undefined,mesh}],interchange:{step}}))
    expect(parsed.interchange?.step?.sceneTree?.products[0]?.name).toBe('Корпус δ')
    expect(parsed.interchange?.step?.display?.faces[0]?.rgba).toEqual([255,0,0,255])
    expect(parsed.interchange?.step?.xde?.[0]?.densityKgPerM3).toBe(7850)
    expect(parsed.interchange?.step?.lod0ShellCacheKeys).toEqual(['b'.repeat(64)])
    const legacy={...step};delete legacy.sceneTree;delete legacy.display;delete legacy.xde;delete legacy.lod0CacheKey;delete legacy.lod0ShellCacheKeys
    expect(()=>parseDirectDocument(stringifyMeshJson({version:1,sketches:[],bodies:[],interchange:{step:legacy}}))).not.toThrow()
    expect(()=>parseDirectDocument(stringifyMeshJson({version:1,sketches:[],bodies:[],interchange:{step:{...step,sceneTree:{kind:'wrong'}}}}))).toThrow('Invalid interchange metadata')
  })

  it('retains exact import and reports optional scene failure as metadata loss',()=>{
    tessellate.mockReturnValueOnce({positions:Float64Array.from(lod0.lod0Shells[0].mesh.positions),indices:Uint32Array.from(lod0.lod0Shells[0].mesh.indices),faceIds:[],topologyFaceIds:[]} as never)
    bridge.mockImplementation(op=>{
      if(op==='brep_nurbs_import_step_scene')throw Error('unsupported scene')
      if(op==='brep_nurbs_import_step_v10')return {model,document,certificate:{capability:'step-interchange/10',complete:true,notes:[]}}
      if(op==='brep_nurbs_import_step_lod0')return lod0
      throw Error(`unexpected op ${op}`)
    })
    const routed=importStepForWorkbench(mixed)
    expect(routed.retainedModel).toBe(model)
    expect(routed.sceneTree).toBeUndefined()
    expect(routed.report.metadataLoss).toContain('STEP scene/XDE metadata unavailable: unsupported scene')
    expect(Array.from(routed.bodies[0].mesh.positions)).toEqual(lod0.lod0Shells[0].mesh.positions)
    expect(tessellate).not.toHaveBeenCalled()
  })
})
