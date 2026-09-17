import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
const read=path=>JSON.parse(readFileSync(path,'utf8'))
const write=(path,value)=>writeFileSync(path,JSON.stringify(value,null,2)+'\n')
const q='docs/qualification'
const matrix=read(`${q}/brep-full-closed-matrix-v13.json`)
matrix.schemaVersion=14;matrix.planId='brep-full-closed-matrix-v14'
matrix.successorOf=`${q}/brep-full-closed-matrix-v13.json`
matrix.admittedOps=matrix.admittedOps.filter(id=>id!=='step-interchange/5')
matrix.pendingQualification.push('step-interchange/5')
matrix.pendingQualification.push('step-interchange/6')
matrix.unresolvedCandidateRows.push('step-interchange/5')
matrix.unresolvedCandidateRows.push('step-interchange/6')
write(`${q}/brep-full-closed-matrix-v14.json`,matrix)
const registry=read(`${q}/brep-capability-registry-release-full-v13.json`)
registry.schemaVersion=14;registry.planId='brep-capability-registry-release-full-v14'
registry.successorOf=`${q}/brep-capability-registry-release-full-v13.json`
registry.closedMatrix=`${q}/brep-full-closed-matrix-v14.json`;registry.g8Index=`${q}/plans/g8-full-matrix-index-v14.json`
registry.capabilities=registry.capabilities.filter(id=>id!=='step-interchange/5')
registry.excludedPendingQualification.push('step-interchange/5')
registry.excludedPendingQualification.push('step-interchange/6')
write(`${q}/brep-capability-registry-release-full-v14.json`,registry)
const index=read(`${q}/plans/g8-full-matrix-index-v13.json`)
index.schemaVersion=14;index.successorOf=`${q}/plans/g8-full-matrix-index-v13.json`
index.matrix=`${q}/brep-full-closed-matrix-v14.json`;index.registry=`${q}/brep-capability-registry-release-full-v14.json`
Object.assign(index.capabilities.find(row=>row.id==='step-interchange/5'),{maturity:'AnalyticComplete',releaseState:'candidate'})
index.capabilities.push({id:'step-interchange/6',plan:`${q}/plans/step-interchange-6.json`,
  evidence:`${q}/step-interchange-6-evidence-v1.json`,dependencies:['persistent-naming/1'],maturity:'ResearchOnly',releaseState:'candidate'})
write(`${q}/plans/g8-full-matrix-index-v14.json`,index)
const paths=[`${q}/brep-full-closed-matrix-v14.json`,`${q}/brep-capability-registry-release-full-v14.json`,`${q}/plans/g8-full-matrix-index-v14.json`]
console.log(createHash('sha256').update(paths.map(path=>readFileSync(path)).join('')).digest('hex'))
