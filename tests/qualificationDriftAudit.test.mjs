import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import test from 'node:test'
import {auditQualificationDrift} from '../scripts/audit-qualification-drift.mjs'

const hash = value => createHash('sha256').update(value).digest('hex')
test('reports all drift without rewriting archives or evidence', () => {
  const root = mkdtempSync(join(tmpdir(), 'qualification-drift-'))
  try {
    mkdirSync(join(root, 'crates'))
    const files = {
      'archive.json': 'archive', 'package.json': 'changed',
      'THIRD_PARTY_NOTICES.md': 'notices', 'package-lock.json': 'lock', 'crates/Cargo.lock': 'rust',
      'kernel.wasm': 'new wasm', 'source.ts': 'new source',
    }
    for (const [path, contents] of Object.entries(files)) writeFileSync(join(root, path), contents)
    const fingerprint = {artifacts: [{path:'package.json',sha256:hash('old'),byteLength:3}]}
    const evidence = {wasm:{path:'kernel.wasm',sha256:hash('old wasm'),byteLength:8},
      dependencyFingerprints:{noticesSha256:hash('notices'),npmLockfileSha256:hash('lock'),rustLockfileSha256:hash('rust')},
      sourceBundle:{paths:['source.ts'],sha256:hash('source.ts\0old source\n')}}
    writeFileSync(join(root,'fingerprint.json'),JSON.stringify(fingerprint))
    writeFileSync(join(root,'evidence.json'),JSON.stringify(evidence))
    const options={fingerprintPath:'fingerprint.json',evidencePath:'evidence.json',archives:{'archive.json':hash('archive')}}
    const report=auditQualificationDrift(root,options)
    assert.equal(report.matches,false)
    assert.equal(report.qualificationClaim,'none')
    assert.deepEqual(report.records.filter(r=>r.status==='mismatch').map(r=>r.kind),['frozen-input','kernel','source-bundle'])
    for(const [path,contents] of Object.entries(files))assert.equal(readFileSync(join(root,path),'utf8'),contents)
    assert.deepEqual(JSON.parse(readFileSync(join(root,'evidence.json'))),evidence)
    rmSync(join(root,'source.ts'))
    const missing=auditQualificationDrift(root,options)
    assert.deepEqual(missing.missingSources,['source.ts'])
    assert.equal(missing.records.at(-1).status,'missing')
    writeFileSync(join(root,'archive.json'),'tampered')
    assert.equal(auditQualificationDrift(root,options).records[0].status,'mismatch')
    for(const [path,contents] of Object.entries({'archive.json':'archive','source.ts':'old source','package.json':'old','kernel.wasm':'old wasm'})) {
      writeFileSync(join(root,path),contents)
    }
    assert.equal(auditQualificationDrift(root,options).matches,true)
    assert.throws(()=>auditQualificationDrift(root,{...options,evidencePath:'missing.json'}),/Missing required evidence document/)
  } finally { rmSync(root,{recursive:true,force:true}) }
})
