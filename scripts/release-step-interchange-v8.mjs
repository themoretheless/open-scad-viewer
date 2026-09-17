import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
const read=path=>JSON.parse(readFileSync(path,'utf8'))
const write=(path,value)=>writeFileSync(path,JSON.stringify(value,null,2)+'\n')
const q='docs/qualification'
const matrix=read(`${q}/brep-full-closed-matrix-v15.json`)
matrix.schemaVersion=16;matrix.planId='brep-full-closed-matrix-v16'
matrix.successorOf=`${q}/brep-full-closed-matrix-v15.json`
matrix.admittedOps.push('step-interchange/8')
write(`${q}/brep-full-closed-matrix-v16.json`,matrix)
const registry=read(`${q}/brep-capability-registry-release-full-v15.json`)
registry.schemaVersion=16;registry.planId='brep-capability-registry-release-full-v16'
registry.successorOf=`${q}/brep-capability-registry-release-full-v15.json`
registry.closedMatrix=`${q}/brep-full-closed-matrix-v16.json`
registry.g8Index=`${q}/plans/g8-full-matrix-index-v16.json`
registry.capabilities.push('step-interchange/8')
write(`${q}/brep-capability-registry-release-full-v16.json`,registry)
const index=read(`${q}/plans/g8-full-matrix-index-v15.json`)
index.schemaVersion=16;index.successorOf=`${q}/plans/g8-full-matrix-index-v15.json`
index.matrix=`${q}/brep-full-closed-matrix-v16.json`
index.registry=`${q}/brep-capability-registry-release-full-v16.json`
index.capabilities.push({id:'step-interchange/8',plan:`${q}/plans/step-interchange-8.json`,
  evidence:`${q}/step-interchange-8-evidence-v12.json`,dependencies:['persistent-naming/1'],
  maturity:'Qualified',releaseState:'shipped'})
write(`${q}/plans/g8-full-matrix-index-v16.json`,index)
const paths=[`${q}/brep-full-closed-matrix-v16.json`,`${q}/brep-capability-registry-release-full-v16.json`,
  `${q}/plans/g8-full-matrix-index-v16.json`]
console.log(createHash('sha256').update(paths.map(path=>readFileSync(path)).join('')).digest('hex'))
