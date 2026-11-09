import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {exportDirectStepV9} from '../src/services/cadNurbsStep'

export interface StepLod0BenchmarkFixture {
  exactText:string
  mixedText:string
  exactMesh:{positions:number[];indices:number[]}
  lod0Mesh:{positions:number[];indices:number[]}
}

/** Deterministic exact AP242 box plus a linked, baked triangle replica. */
export function createStepLod0BenchmarkFixture(zOffset=0):StepLod0BenchmarkFixture {
  const model=createBrepBox([0,0,0],[2,3,4])
  const exactText=exportDirectStepV9(model).text
  const preview=tessellateNurbsBrep(model,8)
  const exactMesh={positions:Array.from(preview.positions),indices:Array.from(preview.indices)}
  const lod0Mesh={positions:exactMesh.positions.map((value,index)=>index%3===2?value+zOffset:value),indices:[...exactMesh.indices]}
  const exactId=exactText.match(/#(\d+)=MANIFOLD_SOLID_BREP\s*\(/)?.[1]
  if(!exactId)throw Error('Fixture export has no authoritative MANIFOLD_SOLID_BREP')
  const ids=[...exactText.matchAll(/#(\d+)/g)].map(match=>Number(match[1]))
  const next=Math.max(...ids)+1
  const real=(value:number)=>Object.is(value,-0)?'0.00000000000000000E+00':`${value.toExponential(17).replace('e','E')}`
  const points=lod0Mesh.positions.reduce<string[]>((rows,_,index)=>{
    if(index%3===0)rows.push(`(${real(lod0Mesh.positions[index])},${real(lod0Mesh.positions[index+1])},${real(lod0Mesh.positions[index+2])})`)
    return rows
  },[]).join(',')
  const triangles=lod0Mesh.indices.reduce<string[]>((rows,_,index)=>{
    if(index%3===0)rows.push(`(${lod0Mesh.indices[index]+1},${lod0Mesh.indices[index+1]+1},${lod0Mesh.indices[index+2]+1})`)
    return rows
  },[]).join(',')
  const fragment=`#${next}=COORDINATES_LIST('',(${points}));\n#${next+1}=TRIANGULATED_FACE('baked',#${next},(${triangles}));\n#${next+2}=TESSELLATED_SHELL('LOD-0',(#${next+1}));\n#${next+3}=GEOMETRIC_MODEL_RELATIONSHIP('','',#${next+2},#${exactId});\n`
  const mixedText=exactText.replace(/ENDSEC;\s*END-ISO-10303-21;\s*$/,`${fragment}ENDSEC;\nEND-ISO-10303-21;\n`)
  if(mixedText===exactText)throw Error('Could not splice LOD-0 into exact AP242 fixture')
  return {exactText,mixedText,exactMesh,lod0Mesh}
}
