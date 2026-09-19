import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'

const read=path=>JSON.parse(readFileSync(path,'utf8'))
const write=(path,value)=>writeFileSync(path,JSON.stringify(value,null,2)+'\n')
const q='docs/qualification'

const matrix=read(`${q}/brep-full-closed-matrix-v12.json`)
matrix.schemaVersion=13
matrix.planId='brep-full-closed-matrix-v13'
matrix.successorOf=`${q}/brep-full-closed-matrix-v12.json`
matrix.admittedOps.push('step-interchange/5')
write(`${q}/brep-full-closed-matrix-v13.json`,matrix)

const registry=read(`${q}/brep-capability-registry-release-full-v12.json`)
registry.schemaVersion=13
registry.planId='brep-capability-registry-release-full-v13'
registry.successorOf=`${q}/brep-capability-registry-release-full-v12.json`
registry.closedMatrix=`${q}/brep-full-closed-matrix-v13.json`
registry.g8Index=`${q}/plans/g8-full-matrix-index-v13.json`
registry.capabilities.push('step-interchange/5')
write(`${q}/brep-capability-registry-release-full-v13.json`,registry)

const index=read(`${q}/plans/g8-full-matrix-index-v12.json`)
index.schemaVersion=13
index.successorOf=`${q}/plans/g8-full-matrix-index-v12.json`
index.matrix=`${q}/brep-full-closed-matrix-v13.json`
index.registry=`${q}/brep-capability-registry-release-full-v13.json`
index.capabilities.push({
  id:'step-interchange/5',
  plan:`${q}/plans/step-interchange-5.json`,
  evidence:`${q}/step-interchange-5-evidence-v12.json`,
  dependencies:['persistent-naming/1'],
  maturity:'Qualified',
  releaseState:'shipped',
})
write(`${q}/plans/g8-full-matrix-index-v13.json`,index)

const paths=[
  `${q}/brep-full-closed-matrix-v13.json`,
  `${q}/brep-capability-registry-release-full-v13.json`,
  `${q}/plans/g8-full-matrix-index-v13.json`,
]
console.log(createHash('sha256').update(paths.map(path=>readFileSync(path)).join('')).digest('hex'))
