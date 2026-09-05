import { expect, it } from 'vitest'
import { compileModelGraph, MODELGRAPH_ASSEMBLY_EXAMPLE } from '../src/services/modelGraph'
import { placeAssembly } from '../src/services/modelGraphAssembly'
import { inspectModelGraphInterference } from '../src/mcp/modelGraphInterference'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'
const frame={origin:[0,0,0] as [number,number,number],rotation:[0,0,0] as [number,number,number]}
it('retains nested leaf identity and correct world sources',async()=>{
 const inner=structuredClone(MODELGRAPH_ASSEMBLY_EXAMPLE)
 const doc={...inner,nodes:[...inner.nodes,{id:'outer',op:'assembly',components:[{id:'module',input:'product',anchors:[],placement:{origin:[10,20,30],rotation:[0,0,90]}}]}],root:'outer'}
 const compiled=compileModelGraph(doc)
 expect(compiled.assembly_components).toHaveLength(3)
 expect(compiled.assembly_components[0]!.is_assembly).toBe(true)
 const lid=compiled.assembly_components.find(c=>c.id==='lid')!
 expect(lid.matrix[11]).toBeCloseTo(35.3)
 expect(lid.parent_path).toContain('/components/module')
 const geometry=new HeadlessGeometryService()
 const built=await geometry.compile(compiled.source,'full'), leaf=await geometry.analyze(lid.source!,'full')
 expect(built.meshes).toHaveLength(2)
 expect(leaf.bounds!.min[2]).toBeCloseTo(35.3,4)
 expect(leaf.bounds!.max[1]).toBeCloseTo(40,4)
})
it('supports limited revolute and slider positions',()=>{
 const base={id:'base',input:'x',anchors:[{id:'a',...frame}],placement:frame}
 const mate={component:'base',anchor:'a',own_anchor:'a',gap:0,rotation:[0,0,0] as [number,number,number]}
 const revolving=placeAssembly([base,{id:'moving',input:'y',anchors:[{id:'a',...frame}],mate:{...mate,joint:{kind:'revolute',position:90,min:0,max:120}}}])
 expect(revolving[1]!.matrix[0]).toBeCloseTo(0)
 expect(revolving[1]!.matrix[4]).toBeCloseTo(1)
 const sliding=placeAssembly([base,{id:'moving',input:'y',anchors:[{id:'a',...frame}],mate:{...mate,joint:{kind:'slider',position:5,min:0,max:10}}}])
 expect(sliding[1]!.matrix[11]).toBe(5)
 expect(()=>placeAssembly([base,{id:'moving',input:'y',anchors:[{id:'a',...frame}],mate:{...mate,joint:{kind:'slider',position:11,min:0,max:10}}}])).toThrow('limits')
})
it('measures overlap in the production kernel and reports separated components',async()=>{
 const runtime=new DirectGeometrySupervisor()
 try {
  const geometry=new HeadlessGeometryService(defaultGeometryBuildEngine,runtime)
  const clear=await inspectModelGraphInterference(MODELGRAPH_ASSEMBLY_EXAMPLE,geometry)
  expect(clear.status).toBe('no_volume_overlap')
  const doc={...MODELGRAPH_ASSEMBLY_EXAMPLE,parameters:[{id:'gap',value:-1}]}
  const overlapping=await inspectModelGraphInterference(doc,geometry)
  expect(overlapping.status).toBe('overlap')
  expect(overlapping.pairs[0]!.intersection_volume_mm3).toBeCloseTo(200,3)
  expect(overlapping.pairs[0]!.method).toBe('manifold_intersection')
 } finally {await runtime.close()}
})
it('does not confuse overlapping bounding boxes with intersecting solids',async()=>{
 const doc={language:'modelgraph/1',units:'mm',parameters:[],nodes:[{id:'ball',op:'sphere',radius:1},{id:'assembly',op:'assembly',components:[{id:'a',input:'ball',anchors:[],placement:frame},{id:'b',input:'ball',anchors:[],placement:{...frame,origin:[1.5,1.5,0]}}]}],root:'assembly'}
 const result=await inspectModelGraphInterference(doc,new HeadlessGeometryService())
 expect(result.status).toBe('no_volume_overlap')
 expect(result.pairs[0]!.method).toBe('manifold_intersection')
})

it('reports unavailable intersections as unknown and enforces the leaf budget',async()=>{
 const doc={...MODELGRAPH_ASSEMBLY_EXAMPLE,parameters:[{id:'gap',value:-1}]}
 const geometry=new HeadlessGeometryService()
 const analyze=geometry.analyze.bind(geometry)
 geometry.analyze=async(source,quality,signal)=>{if(source.startsWith('intersection()'))throw new Error('unavailable');return analyze(source,quality,signal)}
 const result=await inspectModelGraphInterference(doc,geometry)
 expect(result.status).toBe('unknown')
 expect(result.pairs[0]!.intersection_volume_mm3).toBeNull()
 const large={language:'modelgraph/1',units:'mm',parameters:[],nodes:[{id:'ball',op:'sphere',radius:1},{id:'assembly',op:'assembly',components:Array.from({length:9},(_,i)=>({id:`part${i}`,input:'ball',anchors:[],placement:frame}))}],root:'assembly'}
 await expect(inspectModelGraphInterference(large,geometry)).rejects.toThrow('8 leaf')
})
it('evaluates joint parameter expressions through the language compiler',()=>{
 const doc=JSON.parse(JSON.stringify(MODELGRAPH_ASSEMBLY_EXAMPLE))
 const product=doc.nodes.find((n:{id:string})=>n.id==='product')
 product.components[1].mate.joint={kind:'slider',position:{param:'gap'},min:0,max:1}
 const compiled=compileModelGraph(doc)
 expect(compiled.assembly_components[1]!.matrix[11]).toBeCloseTo(5.6)
 expect(compiled.assembly_components[1]!.joint?.position).toBeCloseTo(0.3)
 product.components[1].mate.joint.max=0.1
 expect(()=>compileModelGraph(doc)).toThrow('limits')
})
