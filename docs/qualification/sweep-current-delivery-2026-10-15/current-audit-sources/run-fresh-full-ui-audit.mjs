import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,createWriteStream,unlinkSync} from 'node:fs';
import {spawn,spawnSync} from 'node:child_process';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
import {fullSweepUiJobs} from './sweep-full-ui-jobs.mjs';
const output=resolve(process.argv[2]),inventoryOnly=process.argv.includes('--inventory-only');
const harnessArchive=process.argv.find(a=>a.startsWith('--harness-archive='))?.slice('--harness-archive='.length);
const reusedReports={};
for(const argument of process.argv.filter(a=>a.startsWith('--reuse-report='))){
 const value=argument.slice('--reuse-report='.length),separator=value.indexOf(':');
 assert.ok(separator>0,'Reuse reports must use GROUP:PATH');
 const name=value.slice(0,separator);assert.ok(!Object.hasOwn(reusedReports,name),'Duplicate reused UI group');
 reusedReports[name]=resolve(value.slice(separator+1));
}
mkdirSync(output,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const original=readFileSync('scripts/sweep-full-ui-jobs.mjs','utf8');
const jobs=fullSweepUiJobs;assert.equal(jobs.length,11);
assert.ok(Object.keys(reusedReports).every(name=>jobs.some(([group])=>group===name)),'Unknown reused UI group');
const runner=readFileSync('scripts/check-sweep-miter-matrix-browser.mjs','utf8');
const marker='await mkdir(output,{recursive:true})';assert.equal(runner.split(marker).length,2);
const probe=resolve('scripts/.fresh-ui-inventory-probe.mjs');
writeFileSync(probe,runner.slice(0,runner.indexOf(marker))+"console.log(JSON.stringify({sources,sourceDirectory,fullInventory,plannedCaseKeys,viewports}));\n");
const plan={schema:'sweep-full-ui-inventory/1',passedPreflight:false,inventoryOnly,runnerSha256:sha(runner),canonicalOrchestratorSha256:sha(original),jobs:[]};
try {
 for(const [name,args] of jobs){
  const nativeRunningCancelApplicable=name!=='surface';
  const effectiveArgs=[resolve(output,name),...args,'--gpu-backend=metal',...(nativeRunningCancelApplicable?['--native-running-cancel']:[])];
  const r=spawnSync(process.execPath,[probe,...effectiveArgs],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);
  const inventory=JSON.parse(r.stdout.trim());assert.deepEqual(inventory.fullInventory,inventory.plannedCaseKeys);
  plan.jobs.push({name,args,effectiveArgs,nativeRunningCancelApplicable,sourceDirectory:inventory.sourceDirectory,caseKeys:inventory.fullInventory,sources:inventory.sources.map(file=>({file,sha256:sha(readFileSync(resolve(inventory.sourceDirectory,file)))}))});
 }
} finally {unlinkSync(probe)}
plan.totalCases=plan.jobs.reduce((n,j)=>n+j.caseKeys.length,0);
const baseline=JSON.parse(readFileSync('scripts/sweep-ui-required-baseline.json','utf8'));
assert.equal(Object.values(baseline.groups).flat().length,468);
assert.deepEqual(plan.jobs.map(j=>j.name),Object.keys(baseline.groups));
for(const job of plan.jobs)for(const key of baseline.groups[job.name])assert.ok(job.caseKeys.includes(key),`Missing required original UI case: ${job.name}/${key}`);
plan.baselineSha256=sha(readFileSync('scripts/sweep-ui-required-baseline.json'));
plan.extensions=plan.jobs.flatMap(j=>j.caseKeys.filter(k=>!baseline.groups[j.name].includes(k)).map(key=>({group:j.name,key})));
for(const job of plan.jobs)assert.deepEqual([...job.caseKeys].sort(),[...baseline.groups[job.name],...(baseline.extensions[job.name]??[])].sort(),'Missing or undeclared UI case');
assert.equal(plan.totalCases,478);
plan.nativeObservationsRequired=plan.jobs.filter(j=>j.nativeRunningCancelApplicable).reduce((n,j)=>n+j.caseKeys.length*4,0);assert.equal(plan.nativeObservationsRequired,1472);
if(inventoryOnly){writeFileSync(resolve(output,'inventory-only.json'),JSON.stringify(plan,null,2)+'\n');console.log(`Inventory only:${plan.totalCases} cases,${plan.nativeObservationsRequired} required observations; no runtime qualification`);process.exit(0)}
plan.geometryWasmSha256=sha(readFileSync('public/wasm/geometry-kernel.wasm'));
for(const file of ['src/generated/geometry-kernels/kernel_bg.wasm','dist/wasm/geometry-kernel.wasm'])assert.equal(sha(readFileSync(file)),plan.geometryWasmSha256);
plan.distIndexSha256=sha(readFileSync('dist/index.html'));plan.passedPreflight=true;
writeFileSync(resolve(output,'inventory-plan.json'),JSON.stringify(plan,null,2)+'\n');
const receipt={schema:'sweep-fresh-full-ui-run/1',status:'running',passed:false,jobs:plan.jobs.map(j=>({name:j.name,status:'pending'}))};
const save=()=>writeFileSync(resolve(output,'fresh-run.json'),JSON.stringify(receipt,null,2)+'\n');save();
for(let i=0;i<plan.jobs.length;i++){
 const job=receipt.jobs[i];job.status='running';job.startedAt=new Date().toISOString();save();
 if(Object.hasOwn(reusedReports,job.name)){
  const reportPath=reusedReports[job.name],bytes=readFileSync(reportPath),report=JSON.parse(bytes),required=plan.jobs[i];
  assert.equal(report.passed,true,'Reused UI group must be complete');
  assert.equal(report.headed,false);assert.equal(report.gpuBackendRequested,'metal');
  assert.equal(report.geometryWasmSha256,plan.geometryWasmSha256);assert.equal(report.distIndexSha256,plan.distIndexSha256);
  assert.deepEqual(report.selection.fullInventory,required.caseKeys);assert.deepEqual(report.selection.plannedCaseKeys,required.caseKeys);
  assert.deepEqual(report.cases.map(c=>`${c.viewport.width}:${c.file}`),required.caseKeys);
  if(required.nativeRunningCancelApplicable)assert.equal(report.nativeRunningCancelProbe,true);
  for(const item of report.cases){
   assert.equal(item.status,'passed');assert.deepEqual(item.pageErrors,[]);
   assert.equal(item.sourceSha256,required.sources.find(s=>s.file===item.file)?.sha256);
  }
  for(const [file,expected] of Object.entries(report.sourceHashes)){
   if(sha(readFileSync(file))===expected)continue;
   assert.equal(file,'scripts/check-sweep-miter-matrix-browser.mjs','Changed product source in reused UI group');
   assert.ok(harnessArchive);assert.match(expected,/^[a-f0-9]{64}$/);
   assert.equal(sha(readFileSync(resolve(harnessArchive,`${expected}.mjs`))),expected);
  }
  Object.assign(job,{status:'passed',reusedReport:reportPath,reportSha256:sha(bytes),finishedAt:new Date().toISOString()});save();
  continue;
 }
 const log=createWriteStream(resolve(output,job.name+'.log'));
 const child=spawn(process.execPath,['scripts/check-sweep-miter-matrix-browser.mjs',...plan.jobs[i].effectiveArgs],{stdio:['ignore','pipe','pipe']});child.stdout.pipe(log);child.stderr.pipe(log);
 const code=await new Promise((r,j)=>{child.once('error',j);child.once('close',r)});job.exitCode=code;job.status=code===0?'passed':'failed';job.finishedAt=new Date().toISOString();save();
 if(code!==0){receipt.status='failed-needs-investigation';save();process.exit(1)}
}
const mapPath=resolve(output,'reused-reports.json');writeFileSync(mapPath,JSON.stringify(reusedReports,null,2)+'\n');
const verification=spawnSync(process.execPath,['scripts/verify-full-sweep-ui-audit.mjs',output,resolve(output,'full-reconciliation.json'),`--report-map=${mapPath}`,...(harnessArchive?[`--harness-archive=${harnessArchive}`]:[])],{stdio:'inherit'});receipt.verifierExitCode=verification.status;receipt.passed=verification.status===0;receipt.status=receipt.passed?'passed':'reconciliation-failed';save();process.exit(verification.status??1);
