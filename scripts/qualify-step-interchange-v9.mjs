import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {spawnSync} from 'node:child_process'

const root=resolve(import.meta.dirname,'..')
const run=(command,args,options={})=>{
  const result=spawnSync(command,args,{cwd:root,encoding:'utf8',stdio:'inherit',...options})
  if(result.error||result.status!==0)throw Error(`${command} ${args.join(' ')} failed (${result.status})`)
}
run(process.execPath,['scripts/qualify-step-interchange-v8.mjs'])
const base=resolve(root,process.env.STEP_V8_OUTPUT||'output/qualification/step-interchange-8-v1')
const output=resolve(root,process.env.STEP_V9_OUTPUT||'output/qualification/step-interchange-9-v1')
const validator=resolve(base,'toolchain/stepcode-build/bin/p21read_sdai_ap242')
const sha256=path=>createHash('sha256').update(readFileSync(path)).digest('hex')
const manifest=JSON.parse(readFileSync(resolve(root,'tools/stepcode-ap242-ed4-manifest.json'),'utf8')).validator
const logs=resolve(output,'stepcode-raw-logs')
mkdirSync(logs,{recursive:true})
const environment={...process.env,STEP_AP242_VALIDATOR:validator,
  STEP_AP242_VALIDATOR_SHA256:sha256(validator),STEP_AP242_VALIDATOR_VERSION:manifest.version,
  STEP_AP242_LOG_DIR:logs}
const positives=[
  'tests/fixtures/step-v9/self-authored-ap242-open-shell.step',
  'tests/fixtures/step-v9/self-authored-ap242-cylinder.step',
]
for(const fixture of positives)run(process.execPath,['scripts/validate-step-ap242.mjs',fixture],{env:environment})
const negative='output/qualification/step-interchange-8-v1/negative-mutations/wrong-coordinate-type.step'
run(process.execPath,['scripts/validate-step-ap242.mjs','--expect-reject',negative],{env:environment})
const result={schema:'open-scad-viewer/step-interchange-v9-oracle-run',status:'pass',
  validator:{name:manifest.name,version:manifest.version,sourceCommit:manifest.sourceCommit,
    executableSha256:sha256(validator),schema:manifest.schema},
  positiveFixtures:positives,negativeFixtures:[negative],rawLogDirectory:logs}
writeFileSync(resolve(output,'stepcode-oracle-result.json'),JSON.stringify(result,null,2)+'\n')
console.log(JSON.stringify(result,null,2))
