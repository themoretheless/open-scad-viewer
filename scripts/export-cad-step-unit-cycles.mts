import 'fake-indexeddb/auto'
import assert from 'node:assert/strict'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {emptyDirectDocument} from '../src/services/directModeling'
import {prepareSolidStepImport,exportSolidStepCurrent} from '../src/services/solidStepExchange'
import {pushPullFace,solidTopology} from '../src/services/directSolidTools'
const source=resolve('docs/qualification/cad-roadmap-2026-09-28/cap-step-exchange')
const directory=resolve(process.argv[2]??'/tmp/cad-step-unit-cycles');mkdirSync(directory,{recursive:true})
const original=JSON.parse(readFileSync(resolve(source,'manifest.json'),'utf8'))
const manifest={schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-5,relativeVolumeTolerance:1e-8,parts:[] as object[]}
const area:Record<string,number>={bracket:325,enclosure:264,flange:375*Math.PI}
for(const unit of [{name:'centimetre',scale:10,prefix:'.CENTI.'},{name:'metre',scale:1000,prefix:'$'},{name:'inch',scale:25.4,prefix:null}])for(const part of original.parts){
 const text=readFileSync(resolve(source,part.file),'utf8')
 const declaration=/#(\d+)=\(LENGTH_UNIT\(\)NAMED_UNIT\(\*\)SI_UNIT\(\.MILLI\.,\.METRE\.\)\);/g
 const matches=[...text.matchAll(declaration)];assert.equal(matches.length,1)
 const id=matches[0][1],max=Math.max(...[...text.matchAll(/#(\d+)=/g)].map(m=>Number(m[1])))
 const replacement=unit.prefix===null?`#${max+1}=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));\n#${max+2}=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#${max+1});\n#${id}=(CONVERSION_BASED_UNIT('INCH',#${max+2})LENGTH_UNIT()NAMED_UNIT(*));`:matches[0][0].replace('.MILLI.',unit.prefix)
 const input=text.replace(declaration,replacement),name=part.name+'-'+unit.name
 const expected=structuredClone(part.expected)
 expected.boundsMm=expected.boundsMm.map((p:number[])=>p.map(v=>v*unit.scale));expected.volumeMm3*=unit.scale**3
 const save=(suffix:string,step:string,dimensions:object)=>{const file=name+'-'+suffix+'.step';writeFileSync(resolve(directory,file),step);manifest.parts.push({name:name+'-'+suffix,file,sha256:createHash('sha256').update(step).digest('hex'),expected:structuredClone(dimensions)})}
 save('input',input,expected)
 const imported=await prepareSolidStepImport(input,emptyDirectDocument()),body=imported.document.bodies[0]
 assert.equal(imported.document.bodies.length,1)
 const before=JSON.stringify(body),top=solidTopology(body.mesh).faces.map((face,index)=>({face,index})).filter(({face})=>face.normal[2]>.99).sort((a,b)=>b.face.center[2]-a.face.center[2])[0].index
 const edited=pushPullFace(body,top,1)
 assert.equal(edited.id,body.id);assert.equal(JSON.stringify(body),before)
 expected.boundsMm[1][2]+=1;expected.volumeMm3+=area[part.name]*unit.scale**2
 save('edited',await exportSolidStepCurrent(edited),expected)
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify(manifest,null,2)+'\n')
console.log({cycles:manifest.parts.length/2,independentFiles:manifest.parts.length})
