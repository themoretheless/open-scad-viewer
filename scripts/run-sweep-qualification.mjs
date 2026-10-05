import assert from 'node:assert/strict'
import {readFileSync,existsSync} from 'node:fs'
import {spawnSync} from 'node:child_process'
const root=new URL('../',import.meta.url)
const catalog=JSON.parse(readFileSync(new URL('docs/design/sweep-qualification-catalog.json',root),'utf8'))
assert.equal(catalog.schema,'sweep-qualification-catalog/1')
for(const entry of catalog.step.baseline)assert.ok(['certify','refuse'].includes(entry.nativeAdmission),'Invalid native admission expectation: '+entry.file)
for(const files of [catalog.browser.solid,...Object.values(catalog.step).filter(Array.isArray).filter(a=>a[0]?.file).map(a=>a.map(c=>c.file))])assert.equal(new Set(files).size,files.length,'Duplicate qualification case')
for(const file of catalog.browser.solid)assert.ok(existsSync(new URL('examples/rush/'+file,root)),file)
for(const target of ['wasm','rush'])for(const file of catalog.suites[target])assert.ok(existsSync(new URL(file,root)),file)
const target=process.argv[2]??'list'
const args=process.argv.slice(3)
const commands={
 native:[...catalog.suites.native.filters.map(filter=>['cargo','test','--locked','--manifest-path','crates/Cargo.toml','-p',catalog.suites.native.package,'--lib',filter]),...(catalog.suites.native.bridgeTests??[]).map(test=>['cargo','test','--locked','--manifest-path','crates/Cargo.toml','-p','geometry-bridge','--test',test])],
 wasm:[['node_modules/.bin/vitest','run',...catalog.suites.wasm]],
 rush:[['node_modules/.bin/vitest','run',...catalog.suites.rush]],
 browser:[['node','scripts/check-sweep-miter-matrix-browser.mjs',...args]],
 step:[['node','--import','tsx','scripts/export-sweep-step-oracle.mts',...args]],
 'step-smooth':[['node','--import','tsx','scripts/export-smooth-station-step-oracle.mts',...args],['python3','scripts/reference-sweep-generator-volume.py',args[0]??'/tmp/sweep-smooth-station-step']],
}
if(target==='list')console.log(JSON.stringify({browser:catalog.browser.solid.length,step:catalog.step.baseline.length,smooth:catalog.step.smooth.length,commands},null,2))
else {
 assert.ok(commands[target],'Unknown target: '+target)
 for(const [cmd,...argv] of commands[target]){
  const result=spawnSync(cmd,argv,{cwd:root,stdio:'inherit',env:{...process.env,VITEST_MAX_WORKERS:process.env.VITEST_MAX_WORKERS??'2'}})
  if(result.error)throw result.error
  if(result.status!==0)process.exit(result.status??1)
 }
}
