// Evidence parsing only. No native test selection or geometric computation.
import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {nativeTestCatalog} from '../../../scripts/verify-native-test-partition.mjs'
const [catalogDirectory,serialLog,parallelLog,output]=process.argv.slice(2)
assert(catalogDirectory&&serialLog&&parallelLog&&output)
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
function execution(text){
 const results=new Map();let target
 for(const line of text.split(/\r?\n/u)){
  const run=/\bRunning .*\(([^)]+)\)\s*$/u.exec(line),docs=/^\s*Doc-tests\s+(.+)$/u.exec(line)
  if(run)target=`binary:${run[1]}`
  else if(docs)target=`docs:${docs[1]}`
  else {
   const result=/^test (.+) \.\.\. (ok|ignored|FAILED)(?:[\s,]|$)/u.exec(line)
   if(!result)continue
   assert(target,'Test result without target')
   const name=result[1].replace(/(?: - should panic| - compile fail| - compile)$/u,'')
   const key=`${target}\0${name}`
   assert(!results.has(key),`Duplicate execution: ${key}`)
   results.set(key,result[2])
  }
 }
 return results
}
const checks=[]
for(const [part,log] of [['serial',serialLog],['parallel',parallelLog]]){
 const catalogBytes=readFileSync(`${catalogDirectory}/${part}.log`),logBytes=readFileSync(log)
 const catalog=nativeTestCatalog(catalogBytes.toString('utf8')),actual=execution(logBytes.toString('utf8'))
 const missing=[...catalog].filter(x=>!actual.has(x)),extra=[...actual.keys()].filter(x=>!catalog.has(x))
 assert.deepEqual(missing,[],`Missing ${part} cases`);assert.deepEqual(extra,[],`Unlisted ${part} cases`)
 const count=status=>[...actual.values()].filter(x=>x===status).length
 assert.equal(count('FAILED'),0)
 checks.push({part,catalogCases:catalog.size,executed:actual.size,passed:count('ok'),ignored:count('ignored'),failed:0,
  catalogLogSha256:sha(catalogBytes),executionLogSha256:sha(logBytes),
  casesSha256:sha([...actual.keys()].sort().join('\n')+'\n'),missing:0,extra:0,duplicates:0})
}
const report={schema:'exact-native-execution-cover/1',complete:true,checks,
 totals:{passed:checks.reduce((n,c)=>n+c.passed,0),ignored:checks.reduce((n,c)=>n+c.ignored,0),failed:0},
 semanticG1MatrixQualified:false,qualificationClaim:'none',fullGoalComplete:false}
writeFileSync(output,JSON.stringify(report,null,2)+'\n',{flag:'wx'})
console.log(JSON.stringify(report))
