import {describe,expect,it} from 'vitest'
import {compileModelGraph,hashModelGraphDocument,ModelGraphError,setModelGraphParameters} from '../src/services/modelGraph'
import {compileModelGraphNurbs,hashNurbsDocument} from '../src/services/modelGraphNurbs'
import {modelGraphTextControls} from '../src/services/modelGraphText'
import {prepareGraphRust} from '../src/services/geometryRustKernel'

const sphere=()=>({language:'modelgraph/1',units:'mm',parameters:[],nodes:[{id:'Shape',op:'sphere',radius:1}],root:'Shape'})

describe('Rust runtime host boundary',()=>{
  it('accepts parameterized NURBS meshes within the resolved-value budget',()=>{
    const document={language:'modelgraph/nurbs-1',units:'mm',parameters:[{id:'R',value:2}],nodes:[{
      id:'Mesh',op:'triangle_mesh',vertices:Array.from({length:4500},()=>[{param:'R'},{param:'R'},{param:'R'}]),triangles:[[0,1,2]],
    }],root:'Mesh'}
    expect(JSON.stringify(document).length).toBeLessThan(250000)
    const compiled=compileModelGraphNurbs(document)
    expect(compiled.resolved_document.nodes[0]).toMatchObject({vertices:expect.arrayContaining([[2,2,2]])})
    expect((compiled.resolved_document.nodes[0] as {vertices:unknown[]}).vertices).toHaveLength(4500)
    expect(compiled.document.nodes[0]).toMatchObject({vertices:expect.arrayContaining([[{param:'R'},{param:'R'},{param:'R'}]])})
    expect(compiled.document_sha256).toBe(hashNurbsDocument(compiled.document))
  })

  it('still enforces the native NURBS resolved-value budget',()=>{
    const result=prepareGraphRust('nurbs',{language:'modelgraph/nurbs-1',units:'mm',nodes:[{
      id:'Mesh',op:'triangle_mesh',vertices:Array.from({length:6144},()=>[0,0,0]),triangles:Array.from({length:2048},()=>[0,1,2]),
    }],root:'Mesh'})
    expect(result).toMatchObject({ok:false,error:{code:'value_limit',message:'Maximum 30000 document values.'}})
  })

  it.each([undefined,1n,()=>1,Symbol('radius'),NaN,Infinity])('rejects non-JSON numeric input %s with its exact path',value=>{
    const document={...sphere(),nodes:[{id:'Shape',op:'sphere',radius:value}]}
    expect(prepareGraphRust('graph',document)).toMatchObject({ok:false,error:{code:'invalid_document',path:'/nodes/0/radius'}})
    expect(()=>compileModelGraph(document)).toThrow(ModelGraphError)
  })

  it('does not silently drop undefined optional or unknown object fields',()=>{
    expect(prepareGraphRust('graph',{...sphere(),type_policy:undefined})).toMatchObject({ok:false,error:{code:'invalid_document',path:'/type_policy'}})
    expect(prepareGraphRust('graph',{...sphere(),extra:undefined})).toMatchObject({ok:false,error:{code:'invalid_document',path:'/extra'}})
    expect(prepareGraphRust('graph',undefined)).toMatchObject({ok:false,error:{code:'invalid_document',path:'/'}})
    expect(prepareGraphRust('graph',{...sphere(),extra:{nested:[Symbol()]}})).toMatchObject({ok:false,error:{code:'invalid_document',path:'/extra/nested/0'}})
  })

  it('bounds cycles and pending children before JSON serialization',()=>{
    const cyclic:Record<string,unknown>=sphere();cyclic.self=cyclic
    expect(prepareGraphRust('graph',cyclic)).toMatchObject({ok:false,error:{code:'input_limit'}})
    expect(prepareGraphRust('graph',{...sphere(),extra:new Array(20001)})).toMatchObject({ok:false,error:{code:'input_limit'}})
    expect(compileModelGraph(sphere()).source).toContain('sphere(r=1)')
    // Repeated immutable references are legal; they are not a cycle.
    const q={op:'quantity',value:1,unit:'mm'}
    expect(compileModelGraph({...sphere(),nodes:[{id:'Shape',op:'box',size:[q,q,q]}]}).source).toContain('cube([1,1,1]')
  })

  it('returns a structured failure when user object serialization throws',()=>{
    const document={...sphere(),get extra(){throw new Error('getter failure')}}
    expect(prepareGraphRust('graph',document)).toMatchObject({ok:false,error:{code:'invalid_document',path:'/'}})
  })

  it('preserves revision hashes, checks stale updates and reevaluates parameter expressions',()=>{
    const document={...sphere(),parameters:[{id:'Radius',value:2}],nodes:[
      {id:'Ball',op:'sphere',radius:{param:'Radius'}},
      {id:'Shape',op:'translate',input:'Ball',vector:[{op:'multiply',args:[{param:'Radius'},2]},0,0]},
    ]}
    const before=compileModelGraph(document)
    expect(before.document_sha256).toBe(hashModelGraphDocument(before.document))
    const after=setModelGraphParameters(before.document,before.document_sha256,[{id:'Radius',value:5}])
    expect(before.source).toContain('translate([4,0,0])')
    expect(after.source).toContain('translate([10,0,0])')
    expect(after.source).toContain('sphere(r=5)')
    expect(after.document_sha256).not.toBe(before.document_sha256)
    expect(after.document_sha256).toBe(hashModelGraphDocument(after.document))
    expect(before.document.parameters[0]!.value).toBe(2)
    expect(()=>setModelGraphParameters(after.document,before.document_sha256,[{id:'Radius',value:3}])).toThrow('Document changed')
  })

  it('retains Customizer parameters and errors after runtime constraint failures',()=>{
    const controls=modelGraphTextControls('param radius: 1 range 1..10\nvalidate radius.atLeast(2).message("Too small")\nshow sphere(radius)')
    expect(controls.parameters).toHaveLength(1)
    expect(controls.parameters[0]).toMatchObject({name:'radius',value:1,min:1,max:10})
    expect(controls.errors).toEqual(['Too small'])
  })
})
