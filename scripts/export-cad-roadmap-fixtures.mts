import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
const directory=resolve(process.argv[2]??'output/qualification/cad-roadmap-step')
mkdirSync(directory,{recursive:true})
const parts=[]
for(const fixture of cadRoadmapParts()){
 const text=await exportSolidStepCurrent(fixture.body),file=fixture.body.id+'.step'
 writeFileSync(resolve(directory,file),text)
 parts.push({name:fixture.body.name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:fixture.volume,boundsMm:fixture.bounds}})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(`Exported ${parts.length} current-geometry STEP fixtures to ${directory}`)
