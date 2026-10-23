import 'fake-indexeddb/auto'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {retainedLoftFixture} from '../tests/support/retainedLoftFixture'
import {applySolidSceneEdit} from '../src/services/solidSceneEdit'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
const directory=resolve(process.argv[2]??'/tmp/cad-retained-loft');mkdirSync(directory,{recursive:true})
const sketches=await retainedLoftFixture(),document={version:1 as const,bodies:[],sketches}
writeFileSync(resolve(directory,'project.json'),JSON.stringify(document,null,2)+'\n')
const result=applySolidSceneEdit(document,{operation:'loft',id:'base',ids:['base','top'],createdId:'loft',x:0,y:0,z:0,axis:'z',angle:0,scale:1})
const step=await exportSolidStepCurrent(result.bodies[0]);writeFileSync(resolve(directory,'retained-loft.step'),step)
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[{name:'Retained annular loft',file:'retained-loft.step',sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:560*Math.PI/3,boundsMm:[[-3,-3,0],[11,13,10]]}}]},null,2)+'\n')
