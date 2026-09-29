import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {serializeDirectDocument} from '../src/services/directModeling'
import {exportSolidStepAssembly} from '../src/services/solidStepAssembly'
const directory=resolve(process.argv[2]??'output/qualification/cad-roadmap-assembly')
mkdirSync(directory,{recursive:true})
const fixtures=cadRoadmapParts()
const bodies=fixtures.map((f,i)=>({...f.body,material:{name:'Finish '+i,color:['#cc7744','#22aa88','#4466cc','#cccccc'][i]},group:i<2?'Детали':'Корпус',name:i===0?"Кронштейн 'A' #1":f.body.name}))
writeFileSync(resolve(directory,'scene.json'),serializeDirectDocument({version:1,sketches:[],bodies}))
writeFileSync(resolve(directory,'scene.step'),await exportSolidStepAssembly({version:1,sketches:[],bodies}))
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({parts:fixtures.map((f,i)=>({id:f.body.id,volume:f.volume,bounds:f.bounds,color:bodies[i].material.color})),groups:['Детали','Корпус']},null,2))
console.log(directory)
