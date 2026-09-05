import { expect, it } from 'vitest'
import { placeAssembly, frameMatrix, multiplyFrames, type AssemblyComponent } from '../src/services/modelGraphAssembly'
import { compileModelGraph, setModelGraphParameters, MODELGRAPH_ASSEMBLY_EXAMPLE } from '../src/services/modelGraph'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'
const frame = { origin: [0,0,0] as [number,number,number], rotation: [0,0,0] as [number,number,number] }
export const components: AssemblyComponent[] = [
  {id:'base',input:'body',anchors:[{id:'top',...frame,origin:[0,0,5]}],placement:frame},
  {id:'lid',input:'cover',anchors:[{id:'bottom',...frame}],mate:{component:'base',anchor:'top',own_anchor:'bottom',gap:0.3,rotation:[0,0,0]}},
]
it('positions fixed mates with an explicit gap and keeps independent component meshes',async()=>{
  const compiled=compileModelGraph({language:'modelgraph/1',units:'mm',parameters:[],nodes:[{id:'body',op:'box',size:[20,10,5]},{id:'cover',op:'box',size:[20,10,2]},{id:'part',op:'assembly',components}],root:'part'})
  expect(compiled.assembly_components[1]!.matrix[11]).toBeCloseTo(5.3)
  const runtime=new DirectGeometrySupervisor()
  try {
    const built=await new HeadlessGeometryService(defaultGeometryBuildEngine,runtime).compile(compiled.source,'full')
    expect(built.meshes).toHaveLength(2)
    expect(built.analysis.volume).toBeCloseTo(1400,3)
    expect(built.analysis.bounds!.max[2]).toBeCloseTo(7.3,4)
  } finally {await runtime.close()}
})
it('aligns rotated attachment frames and solves out-of-order mate chains',()=>{
  const input=structuredClone(components)
  input[0]!.placement={origin:[10,20,30],rotation:[0,90,0]}
  input[1]!.anchors[0]!.origin=[2,3,4]
  input[1]!.anchors[0]!.rotation=[15,25,35]
  const before=JSON.stringify(input), result=placeAssembly(input.reverse())
  const base=result.find(c=>c.id==='base')!,lid=result.find(c=>c.id==='lid')!
  const expected=multiplyFrames(base.anchors[0]!.matrix,frameMatrix({origin:[0,0,0.3],rotation:[0,0,0]}))
  lid.anchors[0]!.matrix.forEach((v,i)=>expect(v).toBeCloseTo(expected[i]!,8))
  expect(JSON.stringify(input.reverse())).toBe(before)
})
it('rejects ambiguous placement, cyclic mates and dangling anchors',()=>{
  expect(()=>placeAssembly([...components,components[0]!])).toThrow('unique')
  expect(()=>placeAssembly([{...components[0]!,mate:components[1]!.mate}])).toThrow('exactly one')
  expect(()=>placeAssembly([{...components[0]!,placement:undefined,mate:{component:'lid',anchor:'bottom',own_anchor:'top',gap:0,rotation:[0,0,0]}},components[1]!])).toThrow('Cyclic')
  expect(()=>placeAssembly([components[0]!,{...components[1]!,mate:{...components[1]!.mate!,anchor:'missing'}}])).toThrow('Unknown anchor')
})

it('repositions a mate on parameter edits without changing the previous assembly',()=>{
  const before=compileModelGraph(MODELGRAPH_ASSEMBLY_EXAMPLE)
  const after=setModelGraphParameters(before.document,before.document_sha256,[{id:'gap',value:2}])
  expect(before.assembly_components[1]!.matrix[11]).toBeCloseTo(5.3)
  expect(after.assembly_components[1]!.matrix[11]).toBeCloseTo(7)
})
