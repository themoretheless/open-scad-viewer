import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
const read=path=>JSON.parse(readFileSync(path,'utf8'))
const write=(path,value)=>writeFileSync(path,JSON.stringify(value,null,2)+'\n')
const q='docs/qualification'
const matrix=read(`${q}/brep-full-closed-matrix-v17.json`)
matrix.schemaVersion=18;matrix.planId='brep-full-closed-matrix-v18'
matrix.successorOf=`${q}/brep-full-closed-matrix-v17.json`;matrix.admittedOps.push('step-interchange/10')
write(`${q}/brep-full-closed-matrix-v18.json`,matrix)
const registry=read(`${q}/brep-capability-registry-release-full-v17.json`)
registry.schemaVersion=18;registry.planId='brep-capability-registry-release-full-v18'
registry.successorOf=`${q}/brep-capability-registry-release-full-v17.json`
registry.closedMatrix=`${q}/brep-full-closed-matrix-v18.json`;registry.g8Index=`${q}/plans/g8-full-matrix-index-v18.json`
registry.capabilities.push('step-interchange/10')
write(`${q}/brep-capability-registry-release-full-v18.json`,registry)
const index=read(`${q}/plans/g8-full-matrix-index-v17.json`)
index.schemaVersion=18;index.successorOf=`${q}/plans/g8-full-matrix-index-v17.json`
index.matrix=`${q}/brep-full-closed-matrix-v18.json`;index.registry=`${q}/brep-capability-registry-release-full-v18.json`
index.capabilities.push({id:'step-interchange/10',plan:`${q}/plans/step-interchange-10.json`,
 evidence:`${q}/step-interchange-10-evidence-v12.json`,dependencies:['step-interchange/9','persistent-naming/1'],
 maturity:'Qualified',releaseState:'shipped'})
write(`${q}/plans/g8-full-matrix-index-v18.json`,index)
const paths=[`${q}/brep-full-closed-matrix-v18.json`,`${q}/brep-capability-registry-release-full-v18.json`,`${q}/plans/g8-full-matrix-index-v18.json`]
console.log(createHash('sha256').update(paths.map(path=>readFileSync(path)).join('')).digest('hex'))
