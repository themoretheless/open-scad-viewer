import {mkdtempSync,mkdirSync,copyFileSync,writeFileSync,readFileSync,existsSync,rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {resolve,join} from 'node:path'
import {spawnSync} from 'node:child_process'
import {expect,it} from 'vitest'

it.each([
 ['record-own-cad-evidence.mjs','v1'],
 ['release-own-rust-v9.mjs','v9'],
 ['release-own-rust-v10.mjs','v10'],
 ['release-own-rust-v11.mjs','v11'],
 ['release-own-rust-v12.mjs','v12'],
 ['release-own-rust-v13.mjs','v13'],
])('refuses historical publication before any side effect: %s',(script,version)=>{
 const root=mkdtempSync(join(tmpdir(),'osv-historical-writer-'))
 try{
  for(const path of ['scripts','docs/qualification','src/core'])mkdirSync(resolve(root,path),{recursive:true})
  copyFileSync(new URL(`../scripts/${script}`,import.meta.url),resolve(root,'scripts',script))
  const archive=resolve(root,`docs/qualification/own-rust-cad-${version}.json`)
  const binding=resolve(root,'src/core/ownRustCadEvidence.ts')
  writeFileSync(archive,'historical bytes\n')
  writeFileSync(binding,'current binding\n')
  // No prior-version JSON, kernel or test runner: refusal must precede their use.
  const result=spawnSync(process.execPath,[resolve(root,'scripts',script)],{cwd:root,encoding:'utf8',timeout:5000})
  expect(result.error).toBeUndefined()
  expect(result.status).not.toBe(0)
  expect(result.stderr).toContain(`Historical own-rust-cad-${version} evidence already exists`)
  expect(readFileSync(archive,'utf8')).toBe('historical bytes\n')
  expect(readFileSync(binding,'utf8')).toBe('current binding\n')
  expect(existsSync(resolve(root,'output'))).toBe(false)
 }finally{rmSync(root,{recursive:true,force:true})}
})
