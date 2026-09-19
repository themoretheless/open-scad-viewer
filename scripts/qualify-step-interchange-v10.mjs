import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {spawnSync} from 'node:child_process'

const root=resolve(import.meta.dirname,'..')
const output=resolve(root,process.env.STEP_V10_OUTPUT||'output/qualification/step-interchange-10-v1')
mkdirSync(output,{recursive:true})
const runs=[]
const run=(id,command,args,options={})=>{
 const result=spawnSync(command,args,{cwd:root,encoding:'utf8',...options})
 const log=(result.stdout??'')+(result.stderr??'')
 writeFileSync(resolve(output,`${id}.log`),log)
 if(result.error||result.status!==0)throw Error(`${id} failed (${result.status})`)
 runs.push({id,result:'pass',artifactHash:createHash('sha256').update(log).digest('hex')})
}

run('step9-prerequisite',process.execPath,['scripts/qualify-step-interchange-v9.mjs'],{stdio:'inherit'})
const validator=resolve(root,'output/qualification/step-interchange-8-v1/toolchain/stepcode-build/bin/p21read_sdai_ap242')
const manifest=JSON.parse(readFileSync(resolve(root,'tools/stepcode-ap242-ed4-manifest.json'),'utf8')).validator
const sha256=path=>createHash('sha256').update(readFileSync(path)).digest('hex')
const env={...process.env,STEP_AP242_VALIDATOR:validator,STEP_AP242_VALIDATOR_SHA256:sha256(validator),
 STEP_AP242_VALIDATOR_VERSION:manifest.version,STEP_AP242_LOG_DIR:resolve(output,'stepcode-raw-logs')}
mkdirSync(env.STEP_AP242_LOG_DIR,{recursive:true})
run('stepcode-ap242-ed4',process.execPath,['scripts/validate-step-ap242.mjs','tests/fixtures/step-v6/self-authored-ap242-assembly.step'],{env})
run('native-occurrence-graph','cargo',['test','--locked','--manifest-path','crates/Cargo.toml','-p','brep-core','v10_retains_affine_occurrence_graph_and_detects_mutation'])
run('native-affine-operators','cargo',['test','--locked','--manifest-path','crates/Cargo.toml','-p','brep-core','v6_applies_nonuniform_occurrence_affine_and_refuses_singular'])
run('bridge-wasm-build','npm',['run','build:geometry'])
run('product-roundtrips','npx',['vitest','run','tests/brepStepV10Product.test.ts'])
run('browser-workbench-indexeddb',process.execPath,['tools/browser-qualification/step-v10-workbench-indexeddb.mjs'])
const result={schema:'open-scad-viewer/step-interchange-v10-oracle-run',status:'pass',
 validator:{name:manifest.name,version:manifest.version,sourceCommit:manifest.sourceCommit,
  executableSha256:sha256(validator),schema:manifest.schema},
 runs,scopeAudit:{remainingRequiredGeometryProductStepGaps:[],
  outOfScope:['ambient implicit external access','procedural CSG conversion']}}
writeFileSync(resolve(output,'oracle-result.json'),JSON.stringify(result,null,2)+'\n')
console.log(JSON.stringify(result,null,2))
