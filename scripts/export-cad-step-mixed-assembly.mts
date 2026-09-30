import 'fake-indexeddb/auto'
import assert from 'node:assert/strict'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {emptyDirectDocument} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {exportSolidStepAssembly} from '../src/services/solidStepAssembly'
import {prepareSolidStepImport,exportSolidStepCurrent,exportSolidStepOriginal} from '../src/services/solidStepExchange'
import {pushPullFace,solidTopology} from '../src/services/directSolidTools'
const directory=resolve(process.argv[2]??'/tmp/cad-mixed-assembly');mkdirSync(directory,{recursive:true})
const document=emptyDirectDocument()
for(const [id,min,max] of [['mm',[0,0,0],[2,3,4]],['inch',[10,0,0],[12,3,4]]] as const){const brep=createBrepBox([...min],[...max]);document.bodies.push({id,name:id,brep,mesh:tessellateNurbsBrep(brep,1)})}
let text=await exportSolidStepAssembly(document)
const declarations=[...text.matchAll(/#(\d+)=\(LENGTH_UNIT\(\)NAMED_UNIT\(\*\)SI_UNIT\(\.MILLI\.,\.METRE\.\)\);/g)]
assert.equal(declarations.length,3)
const max=Math.max(...[...text.matchAll(/#(\d+)=/g)].map(m=>Number(m[1]))),id=declarations[2][1]
text=text.replace(declarations[2][0],`#${max+1}=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));\n#${max+2}=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#${max+1});\n#${id}=(CONVERSION_BASED_UNIT('INCH',#${max+2})LENGTH_UNIT()NAMED_UNIT(*));`)
const manifest={schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[] as object[]}
async function qualify(name:string,source:string,offset:number){
const expected={solids:2,boundsMm:[[0,0,0],[12*25.4+offset,3*25.4,4*25.4]],volumeMm3:24+24*25.4**3}
function save(name:string,step:string){writeFileSync(resolve(directory,name+'.step'),step);manifest.parts.push({name,file:name+'.step',sha256:createHash('sha256').update(step).digest('hex'),expected:structuredClone(expected)});writeFileSync(resolve(directory,'manifest.json'),JSON.stringify(manifest,null,2)+'\n')}
save(name+'-input',source)
const imported=await prepareSolidStepImport(source,emptyDirectDocument())
writeFileSync(resolve(directory,name+'-import-report.json'),JSON.stringify(imported.report,null,2)+'\n')
assert.equal(imported.document.bodies.length,1)
const body=imported.document.bodies[0],before=JSON.stringify(body)
const faces=solidTopology(body.mesh).faces.map((face,index)=>({face,index})).filter(({face})=>face.normal[2]>.99).sort((a,b)=>b.face.center[2]-a.face.center[2])
const edited=pushPullFace(body,faces[0].index,1)
assert.equal(await exportSolidStepOriginal(),source)
assert.equal(JSON.stringify(body),before);assert.equal(edited.id,body.id)
assert.equal(edited.brep?.bodies.length,2)
assert.deepEqual(edited.brep?.topologyIds?.bodies,body.brep?.topologyIds?.bodies)
// The lower, millimetre component must retain both exact geometry and naming.
const untouched=body.brep!.vertices.map((vertex,index)=>({vertex,id:body.brep!.topologyIds!.vertices[index]})).filter(({vertex})=>vertex.point[0]<3)
assert.equal(untouched.length,8)
for(const {vertex,id} of untouched){
 const index=edited.brep!.topologyIds!.vertices.indexOf(id)
 assert.ok(index>=0);assert.deepEqual(edited.brep!.vertices[index],vertex)
}
expected.boundsMm[1][2]+=1;expected.volumeMm3+=6*25.4**2
save(name+'-edited',await exportSolidStepCurrent(edited))
}
await qualify('mixed',text,0)
// Distinct placement frames: source X=1 inch, target X=100 mm.
const sourcePoint=max+3,sourceFrame=max+4,targetPoint=max+5,targetFrame=max+6,placement=max+7
const placements=[...text.matchAll(/REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION\(#(\d+)\)/g)]
assert.equal(placements.length,2)
const match=placements[1]
let placed=text.slice(0,match.index)+match[0].replace('#'+match[1],'#'+placement)+text.slice(match.index!+match[0].length)
placed=placed.replace('ENDSEC;\nEND-ISO-10303-21;',`#${sourcePoint}=CARTESIAN_POINT('',(1.,0.,0.));\n#${sourceFrame}=AXIS2_PLACEMENT_3D('',#${sourcePoint},$,$);\n#${targetPoint}=CARTESIAN_POINT('',(100.,0.,0.));\n#${targetFrame}=AXIS2_PLACEMENT_3D('',#${targetPoint},$,$);\n#${placement}=ITEM_DEFINED_TRANSFORMATION('','',#${sourceFrame},#${targetFrame});\nENDSEC;\nEND-ISO-10303-21;`)
assert.ok(placed.includes(`#${placement}=ITEM_DEFINED_TRANSFORMATION`))
await qualify('placed',placed,100-25.4)
console.log({parts:manifest.parts.length})
