import assert from 'node:assert/strict'
import {test} from 'node:test'
import {auditArchitecture, layerViolations, nextBaseline, measureSizes} from '../scripts/audit-architecture.mjs'

test('new oversized file fails, pinned file may shrink but not grow', () => {
  const baseline = {sizes: {'a.rs': 900}, layers: []}
  assert.deepEqual(auditArchitecture({sizes: {'a.rs': 850}, layers: []}, baseline), [])
  assert.match(auditArchitecture({sizes: {'a.rs': 901}, layers: []}, baseline)[0], /grew/)
  assert.match(auditArchitecture({sizes: {'a.rs': 850, 'b.ts': 600}, layers: []}, baseline)[0], /b\.ts.*split/)
  assert.match(auditArchitecture({sizes: {}, layers: []}, baseline)[0], /remove it/)
})

test('upward crate dependency is a layering violation', () => {
  assert.deepEqual(layerViolations([['math-core', 'compute-core'], ['brep-core', 'math-core']]), ['math-core -> compute-core'])
  assert.deepEqual(layerViolations([['new-crate', 'math-core']]), ['new-crate: crate has no layer'])
  assert.match(auditArchitecture({sizes: {}, layers: ['x -> y']}, {sizes: {}, layers: []})[0], /upward/)
})

test('update only lowers pins and drops fixed violations', () => {
  const baseline = {sizes: {'a.rs': 900, 'b.rs': 1000}, layers: ['x -> y', 'p -> q']}
  assert.deepEqual(nextBaseline({sizes: {'a.rs': 850, 'c.rs': 999}, layers: ['x -> y']}, baseline),
    {sizes: {'a.rs': 850, 'c.rs': 999}, layers: ['x -> y']})
})


test('historical executor exemption requires exact archived bytes, including short edits', async () => {
  const {mkdtempSync,mkdirSync,readFileSync,writeFileSync,rmSync}=await import('node:fs')
  const {tmpdir}=await import('node:os')
  const {resolve,dirname}=await import('node:path')
  const root=mkdtempSync(resolve(tmpdir(),'architecture-archive-'))
  const path='scripts/refresh-qualification-fingerprints-v49-v66.mjs'
  try {
    mkdirSync(dirname(resolve(root,path)),{recursive:true})
    writeFileSync(resolve(root,path),readFileSync(new URL('../'+path,import.meta.url)))
    assert.deepEqual(measureSizes(root,[path]),{})
    writeFileSync(resolve(root,path),'// shortened forged archive\n')
    assert.throws(()=>measureSizes(root,[path]),/archive hash changed/)
  } finally {rmSync(root,{recursive:true,force:true})}
})
