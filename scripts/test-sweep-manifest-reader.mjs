import {test} from 'node:test'
import assert from 'node:assert/strict'
import {mkdtempSync,writeFileSync,rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {readSweepManifest} from './sweep-manifest-reader.mjs'

test('partition preserves all ordered cases, unknown metadata and JSON numeric values',()=>{
 const dir=mkdtempSync(join(tmpdir(),'sweep-reader-test-'))
 try{
  const path=join(dir,'manifest.json')
  const bytes=Buffer.from('{"metadata":{"label":"шов 🧩","unknown":[false,null,"\\\"cases\\\""]},"cases":[{"number":-0,"values":[9007199254740993,1e-300,1.7976931348623157e308]},{"nested":{"cases":[3,2,1]},"identity":"second"}],"tail":{"proof":false}}')
  writeFileSync(path,bytes)
  const expected=JSON.parse(bytes.toString())
  assert.deepEqual(readSweepManifest(path,bytes),expected)
  assert.deepEqual(readSweepManifest(path,bytes,{partition:true}),expected)
  assert.equal(Object.is(readSweepManifest(path,bytes,{partition:true}).cases[0].number,-0),true)
  assert.throws(()=>readSweepManifest(path,Buffer.from('{"cases":[]}'),{partition:true}),/Manifest changed while partitioning/)
 }finally{rmSync(dir,{recursive:true,force:true})}
})
