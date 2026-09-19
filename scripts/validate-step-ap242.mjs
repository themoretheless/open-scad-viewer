import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,statSync,writeFileSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
import {basename,join} from 'node:path'
import {tmpdir} from 'node:os'

const manifest=JSON.parse(readFileSync(new URL('../tools/stepcode-ap242-ed4-manifest.json',import.meta.url),'utf8'))
const args=process.argv.slice(2)
const expectReject=args[0]==='--expect-reject'
const fixtures=expectReject?args.slice(1):args
if(!fixtures.length){
  console.error('usage: node scripts/validate-step-ap242.mjs <fixture.step> [...]')
  process.exit(2)
}
const executable=process.env.STEP_AP242_VALIDATOR
const expectedHash=process.env.STEP_AP242_VALIDATOR_SHA256
const version=process.env.STEP_AP242_VALIDATOR_VERSION
const logDirectory=process.env.STEP_AP242_LOG_DIR
if(!executable||!expectedHash||!version){
  console.error(JSON.stringify({status:'not-checked',reason:'validator-unavailable',manifestStatus:manifest.status}))
  process.exit(2)
}
if(!/^[0-9a-f]{64}$/.test(expectedHash)||!statSync(executable).isFile()){
  console.error(JSON.stringify({status:'adapter-error',reason:'invalid-validator-binding'}))
  process.exit(2)
}
const actualHash=createHash('sha256').update(readFileSync(executable)).digest('hex')
if(actualHash!==expectedHash){
  console.error(JSON.stringify({status:'adapter-error',reason:'validator-digest-mismatch',expectedHash,actualHash}))
  process.exit(2)
}
if(version!==manifest.validator?.version){
  console.error(JSON.stringify({status:'adapter-error',reason:'validator-version-mismatch',
    expected:manifest.validator?.version,actual:version}))
  process.exit(2)
}
if(logDirectory)mkdirSync(logDirectory,{recursive:true})
for(const fixture of fixtures){
  const output=join(tmpdir(),`open-scad-viewer-${createHash('sha256').update(fixture).digest('hex')}.step`)
  const run=spawnSync(executable,[fixture,output],{
    encoding:'utf8',stdio:['ignore','pipe','pipe'],
  })
  const status=run.status===0?'externally-conformant':run.status===1?'externally-rejected':'adapter-error'
  const record={fixture,status,validator:{name:manifest.validator.name,version,sha256:actualHash,
    sourceCommit:manifest.validator.sourceCommit,schemaSha256:manifest.validator.schema.sha256},
    exitCode:run.status,stdout:run.stdout,stderr:run.stderr}
  if(logDirectory){
    const stem=basename(fixture).replace(/[^A-Za-z0-9_.-]/g,'_')
    const fixtureId=createHash('sha256').update(fixture).digest('hex').slice(0,12)
    writeFileSync(join(logDirectory,`${fixtureId}-${stem}.json`),JSON.stringify(record,null,2)+'\n')
  }
  console.log(JSON.stringify(record))
  if(expectReject&&run.status===1)continue
  if((expectReject&&run.status===0)||(!expectReject&&run.status!==0)){
    process.exit(run.status!==0&&run.status!==1?2:1)
  }
}
