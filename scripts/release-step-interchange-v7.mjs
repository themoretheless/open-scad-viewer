import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
const read=path=>JSON.parse(readFileSync(path,'utf8'))
const write=(path,value)=>writeFileSync(path,JSON.stringify(value,null,2)+'\n')
const q='docs/qualification'
const matrix=read(`${q}/brep-full-closed-matrix-v14.json`)
matrix.schemaVersion=15;matrix.planId='brep-full-closed-matrix-v15'
matrix.successorOf=`${q}/brep-full-closed-matrix-v14.json`
matrix.pendingQualification.push('step-interchange/7')
matrix.unresolvedCandidateRows.push('step-interchange/7')
write(`${q}/brep-full-closed-matrix-v15.json`,matrix)
const registry=read(`${q}/brep-capability-registry-release-full-v14.json`)
registry.schemaVersion=15;registry.planId='brep-capability-registry-release-full-v15'
registry.successorOf=`${q}/brep-capability-registry-release-full-v14.json`
registry.closedMatrix=`${q}/brep-full-closed-matrix-v15.json`;registry.g8Index=`${q}/plans/g8-full-matrix-index-v15.json`
registry.excludedPendingQualification.push('step-interchange/7')
write(`${q}/brep-capability-registry-release-full-v15.json`,registry)
const index=read(`${q}/plans/g8-full-matrix-index-v14.json`)
index.schemaVersion=15;index.successorOf=`${q}/plans/g8-full-matrix-index-v14.json`
index.matrix=`${q}/brep-full-closed-matrix-v15.json`;index.registry=`${q}/brep-capability-registry-release-full-v15.json`
index.capabilities.push({id:'step-interchange/7',plan:`${q}/plans/step-interchange-7.json`,
  evidence:`${q}/step-interchange-7-evidence-v12.json`,dependencies:['persistent-naming/1'],maturity:'ResearchOnly',releaseState:'candidate'})
write(`${q}/plans/g8-full-matrix-index-v15.json`,index)
const paths=[`${q}/brep-full-closed-matrix-v15.json`,`${q}/brep-capability-registry-release-full-v15.json`,`${q}/plans/g8-full-matrix-index-v15.json`]
console.log(createHash('sha256').update(paths.map(path=>readFileSync(path)).join('')).digest('hex'))
