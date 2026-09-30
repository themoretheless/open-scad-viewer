import {readFileSync,writeFileSync,mkdirSync,existsSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {resolve} from 'node:path'
import {expect,it} from 'vitest'
import {inspectNurbsBrep,analyzeNurbsBrep} from '../src/services/geometry/brep'

// Verify real browser exports against independently checked service histories.
const root=process.env.CAD_MIXED_UI_EVIDENCE
const fixtures=process.env.CAD_MIXED20_UI_OUTPUT
it.runIf(!!root&&!!fixtures).each((process.env.CAD_MIXED_UI_PARTS??'bracket,enclosure,flange').split(','))('validates every exported UI solid for %s',async name=>{
 const expected=JSON.parse(readFileSync(resolve(fixtures!,name,'expected.json'),'utf8'))
 for(let step=0;step<=20;step++){
  const document=JSON.parse(readFileSync(resolve(root!,`${name}-${step}.json`),'utf8'))
  const body=document.bodies.find((b:any)=>b.id===name),oracle=expected[step].bodies[0]
  expect(inspectNurbsBrep(body.brep).topologyValid,`step ${step}`).toBe(true)
  const actual=analyzeNurbsBrep(body.brep),reference=analyzeNurbsBrep(oracle.brep)
  expect(actual.signedVolumeMm3,`step ${step}: volume`).toBeCloseTo(reference.signedVolumeMm3,4)
  const used=Math.min(6,Math.floor((step+1)/3))
  expect(document.bodies).toHaveLength(7-used)
  for(let tool=used;tool<6;tool++){
   const initial=JSON.parse(readFileSync(resolve(fixtures!,name,`tool-${tool}.json`),'utf8'))
   expect(document.bodies.find((b:any)=>b.id===initial.id)?.brep,`step ${step}: untouched tool ${tool}`).toEqual(initial.brep)
  }
 }
 const output=process.env.CAD_MIXED_UI_STEP_OUTPUT
 if(output){
  const {exportSolidStepCurrent}=await import('../src/services/solidStepExchange')
  const {importDirectStepV9}=await import('../src/services/cadNurbsStep')
  const final=JSON.parse(readFileSync(resolve(root!,`${name}-20.json`),'utf8')).bodies.find((b:any)=>b.id===name)
  const first=await exportSolidStepCurrent(final)
  const second=await exportSolidStepCurrent({...final,brep:importDirectStepV9(first).model})
  mkdirSync(output,{recursive:true});const manifestPath=resolve(output,'manifest.json')
  const manifest=existsSync(manifestPath)?JSON.parse(readFileSync(manifestPath,'utf8')):{schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[]}
  const oracle=expected[20].bodies[0].brep
  const boundsMm=[Math.min,Math.max].map(fn=>[0,1,2].map(a=>fn(...oracle.vertices.map((v:any)=>v.point[a]))))
  const volumeMm3=analyzeNurbsBrep(oracle).signedVolumeMm3
  manifest.parts=manifest.parts.filter((p:any)=>!p.name.startsWith(name+'-ui20-'))
  for(const [i,text] of [first,second].entries()){
   const part=name+'-ui20-'+i,file=part+'.step';writeFileSync(resolve(output,file),text)
   manifest.parts.push({name:part,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3,boundsMm}})
  }
  writeFileSync(manifestPath,JSON.stringify(manifest,null,2))
 }
},60000)
