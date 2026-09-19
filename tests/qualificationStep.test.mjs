import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdtempSync,readFileSync,rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {test} from 'node:test'
import {runQualificationStep} from '../scripts/qualification-step.mjs'

test('successful child retains the exact stdout/stderr artifact hash',()=>{
 const output=mkdtempSync(join(tmpdir(),'qualification-step-'))
 try{
  const result=runQualificationStep({id:'pass',command:process.execPath,
   args:['-e','process.stdout.write("out");process.stderr.write("err")'],cwd:output,output})
  assert.deepEqual(result,{id:'pass',result:'pass',artifactHash:createHash('sha256').update('outerr').digest('hex')})
  assert.equal(readFileSync(join(output,'pass.log'),'utf8'),'outerr')
 }finally{rmSync(output,{recursive:true,force:true})}
})

test('failure keeps full logs and prints only a bounded tail with exit status',()=>{
 const output=mkdtempSync(join(tmpdir(),'qualification-step-'))
 const write=process.stderr.write
 let consoleLog=''
 process.stderr.write=function(chunk){consoleLog+=chunk;return true}
 try{
  assert.throws(()=>runQualificationStep({id:'fail',command:process.execPath,
   args:['-e','process.stdout.write("x".repeat(20000));process.stderr.write("root cause");process.exitCode=7'],cwd:output,output}),/exit 7/)
  assert.equal(readFileSync(join(output,'fail.log'),'utf8'),'x'.repeat(20000)+'root cause')
  assert.ok(consoleLog.endsWith('root cause\n'))
  assert.ok(consoleLog.includes('exit 7'))
  assert.ok(consoleLog.length<17000)
 }finally{process.stderr.write=write;rmSync(output,{recursive:true,force:true})}
})

test('spawn failure retains the underlying cause instead of only null status',()=>{
 const output=mkdtempSync(join(tmpdir(),'qualification-step-'))
 const write=process.stderr.write
 let consoleLog=''
 process.stderr.write=function(chunk){consoleLog+=chunk;return true}
 try{
  assert.throws(()=>runQualificationStep({id:'missing',command:join(output,'missing'),args:[],cwd:output,output}),
   error=>error.cause?.code==='ENOENT'&&error.message.includes('ENOENT'))
  assert.ok(consoleLog.includes('ENOENT'))
  assert.equal(readFileSync(join(output,'missing.log'),'utf8'),'')
 }finally{process.stderr.write=write;rmSync(output,{recursive:true,force:true})}
})

test('output-buffer failure is not mislabeled as an ordinary exit',()=>{
 const output=mkdtempSync(join(tmpdir(),'qualification-step-'))
 const write=process.stderr.write
 process.stderr.write=()=>true
 try{
  assert.throws(()=>runQualificationStep({id:'overflow',command:process.execPath,
   args:['-e','process.stdout.write("x".repeat(100000))'],cwd:output,output,options:{maxBuffer:1024}}),
   error=>error.cause?.code==='ENOBUFS'&&error.message.includes('ENOBUFS'))
  assert.ok(readFileSync(join(output,'overflow.log')).length>0)
 }finally{process.stderr.write=write;rmSync(output,{recursive:true,force:true})}
})
