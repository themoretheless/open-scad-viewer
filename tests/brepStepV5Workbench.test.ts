import {describe,expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {createBrepBox} from '../src/services/geometry/brep'
import {exportDirectStepV5,importDirectStepV5} from '../src/services/cadNurbsStep'
import {exportRetainedStepForWorkbench,importStepForWorkbench,inspectStepRoute,retainedStepSession} from '../src/services/cadStepRouting'
import {cadOperation,type CadOptions} from '../src/services/cadWorkbench'

const values=new Map<string,string>()
Object.defineProperty(globalThis,'localStorage',{configurable:true,value:{
  getItem:(key:string)=>values.get(key)??null,
  setItem:(key:string,value:string)=>{values.set(key,value)},
  removeItem:(key:string)=>{values.delete(key)},
  key:(index:number)=>[...values.keys()][index]??null,
  get length(){return values.size},
}})

describe('STEP /5 retained workbench route',()=>{
  it('roundtrips selected AP242 and preserves the authoritative model',()=>{
    const source=createBrepBox([0,0,0],[2,3,4])
    const exported=exportDirectStepV5(source)
    expect(exported.text).toContain("FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'))")
    expect(exported.text).toContain('SHAPE_DEFINITION_REPRESENTATION')
    expect(exported.text).toContain('GLOBAL_UNIT_ASSIGNED_CONTEXT')
    const imported=importDirectStepV5(exported.text)
    expect(imported.identity.preserved).toBe(true)
    expect(imported.model.topologyIds?.faces).toEqual(source.topologyIds?.faces)

    const routed=importStepForWorkbench(exported.text)
    expect(routed.report.route).toBe('retained-ap242-brep')
    expect(routed.report.retained).toBe(true)
    expect(routed.bodies[0].mesh.indices.length).toBeGreaterThan(0)
    expect(routed.bodies[0].brep).toEqual(routed.retainedModel)
    retainedStepSession.set(routed.retainedModel,routed.report)
    expect(exportRetainedStepForWorkbench(retainedStepSession.get()!).certificate.capability).toBe('step-interchange/9')
    expect(routed.report.definitionIdentities).toHaveLength(1)
    expect(routed.report.occurrenceIdentities).toHaveLength(1)
    expect(retainedStepSession.body({...routed.bodies[0],brep:undefined}).brep).toEqual(routed.retainedModel)
    expect(retainedStepSession.reload()).toEqual(routed.retainedModel)
    expect(retainedStepSession.report()?.route).toBe('retained-ap242-brep')
    const options:CadOptions={action:'resize',ids:[routed.bodies[0].id],sketches:[],axis:[0,0,1],origin:[0,0,0],
      amount:0,count:1,width:4,height:6,depth:8,pitch:1,secondary:1,mode:'min',pathId:'',profileIds:[]}
    const edited=cadOperation({version:1,sketches:[],bodies:routed.bodies},options).bodies[0]
    expect(edited.brep).toBeDefined()
    retainedStepSession.set(edited.brep)
    const saved=exportRetainedStepForWorkbench(retainedStepSession.reload()!)
    expect(importDirectStepV5(saved.text).model.bodies).toHaveLength(1)
  })

  it('keeps faceted routing explicit and refuses malformed private graphs',()=>{
    expect(inspectStepRoute('ISO-10303-21;DATA;#1=FACETED_BREP();ENDSEC;END-ISO-10303-21;')).toBe('faceted-brep')
    expect(()=>importDirectStepV5('ISO-10303-21;DATA;#1=MANIFOLD_SOLID_BREP();ENDSEC;END-ISO-10303-21;')).toThrow()
    const component=readFileSync('src/features/CadWorkbenchPanel.vue','utf8')
    expect(component).toContain('Retained AP242 B-rep')
    expect(component).toContain('@change="retainedStepImport"')
    expect(component).toContain('@click="retainedStepExport"')
    expect(component).toContain('FACETED_BREP only')
  })

  it('routes AP242 tessellation and reports semantic CSG without mesh relabeling',()=>{
    const tessellated=`ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('tessellated'),'2;1');
FILE_NAME('t.step','',(''),(''),'','','');
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));
ENDSEC;
DATA;
#1=CARTESIAN_POINT_LIST_3D('',((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)));
#2=TRIANGULATED_FACE_SET('',#1,3,$,.F.,((1,2,3)),$);
#3=TESSELLATED_SHAPE_REPRESENTATION('',(#2),$);
ENDSEC;
END-ISO-10303-21;`
    expect(inspectStepRoute(tessellated)).toBe('tessellated-shape')
    const routed=importStepForWorkbench(tessellated)
    expect(routed.report.route).toBe('tessellated-shape')
    expect(routed.report.retained).toBe(false)
    expect(routed.bodies[0].mesh.indices).toEqual([0,1,2])
    const csg=tessellated.replace(
      "#3=TESSELLATED_SHAPE_REPRESENTATION('',(#2),$);",
      "#3=CONSTRUCTIVE_SOLID_GEOMETRY_REPRESENTATION('',(#4),$);#4=CSG_SOLID('',#5);",
    )
    expect(inspectStepRoute(csg)).toBe('semantic-csg')
    expect(()=>importStepForWorkbench(csg)).toThrow(/no explicit semantic evaluator.*no mesh relabeling/i)
  })

  it('imports complete self-authored rational, seam-carrier, and void fixtures',()=>{
    for(const name of ['cylinder','cone','torus','void','multiple-void']){
      const text=readFileSync(`tests/fixtures/step-v5/self-authored-ap242-${name}.step`,'utf8')
      const imported=importDirectStepV5(text)
      expect(imported.model.bodies.length,name).toBe(1)
      expect(exportDirectStepV5(imported.model).text,name).toContain('SHAPE_DEFINITION_REPRESENTATION')
      if(name==='void')expect(imported.model.faces.some(face=>face.holes.length>0)).toBe(true)
      else if(name==='multiple-void')expect(imported.model.bodies[0].innerShells.length).toBe(2)
      else expect(text,name).toContain('RATIONAL_B_SPLINE_')
    }
    const nested=importDirectStepV5(readFileSync('tests/fixtures/step-v5/self-authored-ap242-nested-placement.step','utf8')).model
    expect(nested.bodies).toHaveLength(2)
    expect(Math.max(...nested.vertices.map(vertex=>vertex.point[1]))).toBeGreaterThan(20)
    const metre=importDirectStepV5(readFileSync('tests/fixtures/step-v5/self-authored-ap242-metre.step','utf8')).model
    const inch=importDirectStepV5(readFileSync('tests/fixtures/step-v5/self-authored-ap242-inch.step','utf8')).model
    expect(Math.max(...metre.vertices.map(vertex=>vertex.point[0]))).toBeGreaterThan(1000)
    expect(Math.max(...inch.vertices.map(vertex=>vertex.point[0]))).toBeCloseTo(50.8,8)
  })
})
