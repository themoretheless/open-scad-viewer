import assert from 'node:assert/strict'
import {test} from 'node:test'
import {auditArchitecture, layerViolations, nextBaseline, measureSizes, mechanicalNames} from '../scripts/audit-architecture.mjs'
import {scanModules, unreachableRustFiles} from '../scripts/rustModuleTree.mjs'

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
    {sizes: {'a.rs': 850, 'c.rs': 999}, layers: ['x -> y'], unreachable: [], mechanical: [], functions: {}, moduleLayers: [], srcLayers: []})
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

test('module scanner follows path attributes, inline modules and visibility, and skips comments', () => {
  const {mods, includes} = scanModules([
    '// mod commented;', '/* mod blocked; */', 'const S: &str = "mod quoted;";',
    '#[cfg(test)]', '#[path = "x/y.rs"]', 'pub(in crate::a) mod tests;',
    'mod outer {', '    mod nested;', '}', 'pub(crate) mod plain;', 'include!("gen.rs");',
  ].join('\n'))
  assert.deepEqual(mods, [
    {name: 'tests', path: 'x/y.rs', inline: []},
    {name: 'nested', path: null, inline: ['outer']},
    {name: 'plain', path: null, inline: []},
  ])
  assert.deepEqual(includes, ['gen.rs'])
})

test('a Rust file outside every module tree is reported', async () => {
  const {mkdtempSync, mkdirSync, writeFileSync, rmSync} = await import('node:fs')
  const {tmpdir} = await import('node:os')
  const {resolve} = await import('node:path')
  const root = mkdtempSync(resolve(tmpdir(), 'architecture-tree-'))
  try {
    const write = (path, text) => { mkdirSync(resolve(root, path, '..'), {recursive: true}); writeFileSync(resolve(root, path), text) }
    write('crates/k/Cargo.toml', '[package]\nname = "k"\n')
    write('crates/k/src/lib.rs', 'mod a;\n#[path = "deep/b.rs"]\nmod b;\n')
    write('crates/k/src/a.rs', 'mod c;\n')
    write('crates/k/src/a/c.rs', '')
    write('crates/k/src/deep/b.rs', '')
    write('crates/k/src/stale.rs', '')
    write('crates/k/tests/it.rs', '')
    const files = ['lib.rs', 'a.rs', 'a/c.rs', 'deep/b.rs', 'stale.rs'].map(f => 'crates/k/src/' + f).concat('crates/k/tests/it.rs')
    assert.deepEqual(unreachableRustFiles(root, files), ['crates/k/src/stale.rs'])
  } finally { rmSync(root, {recursive: true, force: true}) }
})

test('uncompiled files and mechanical names are pinned lists that may only shrink', () => {
  const baseline = {sizes: {}, layers: [], unreachable: ['crates/k/src/old.rs'], mechanical: ['crates/k/src/methods_a.rs']}
  assert.deepEqual(auditArchitecture({sizes: {}, layers: [], unreachable: ['crates/k/src/old.rs'], mechanical: ['crates/k/src/methods_a.rs']}, baseline), [])
  assert.match(auditArchitecture({sizes: {}, layers: [], unreachable: ['crates/k/src/old.rs', 'crates/k/src/new.rs'], mechanical: ['crates/k/src/methods_a.rs']}, baseline)[0], /new\.rs: not compiled/)
  assert.match(auditArchitecture({sizes: {}, layers: [], unreachable: [], mechanical: ['crates/k/src/methods_a.rs']}, baseline)[0], /old\.rs: no longer/)
  assert.deepEqual(mechanicalNames(['crates/k/src/methods_Foo_bar.rs', 'crates/k/src/eval_cases_2.rs', 'crates/k/src/compiler_impl_3.rs', 'crates/k/src/methods.rs', 'src/methods_x.rs']),
    ['crates/k/src/compiler_impl_3.rs', 'crates/k/src/eval_cases_2.rs', 'crates/k/src/methods_Foo_bar.rs'])
})

test('long functions are pinned and may only shrink', async () => {
  const {rustFunctions} = await import('../scripts/functionSizes.mjs')
  assert.deepEqual(rustFunctions('fn a() {\n  let s = "}";\n}\ntrait T { fn d(); fn e() {} }\n'),
    [{name: 'a', line: 1, lines: 3}, {name: 'e', line: 4, lines: 1}])
  const baseline = {sizes: {}, layers: [], functions: {'k.rs::f@1': 300}}
  assert.deepEqual(auditArchitecture({sizes: {}, layers: [], functions: {'k.rs::f@1': 280}}, baseline), [])
  assert.match(auditArchitecture({sizes: {}, layers: [], functions: {'k.rs::f@1': 301}}, baseline)[0], /grew/)
  assert.match(auditArchitecture({sizes: {}, layers: [], functions: {'k.rs::f@1': 280, 'k.rs::g@1': 151}}, baseline)[0], /g@1: function has 151 lines/)
  assert.match(auditArchitecture({sizes: {}, layers: [], functions: {}}, baseline)[0], /within limits or gone/)
  assert.deepEqual(nextBaseline({sizes: {}, layers: [], functions: {'k.rs::f@1': 280, 'k.rs::g@1': 200}}, baseline).functions, {'k.rs::f@1': 280})
})

test('clippy warnings are counted once per location and may only go down', async () => {
  const {countClippy, auditClippy} = await import('../scripts/audit-clippy.mjs')
  const warning = (file, line, code = 'clippy::x') => JSON.stringify({reason: 'compiler-message', message: {level: 'warning', code: {code},
    message: 'm', spans: [{is_primary: true, file_name: file, line_start: line, column_start: 1}]}})
  const out = [warning('k/src/a.rs', 1), warning('k/src/a.rs', 1), warning('k/src/a.rs', 2), warning('/rustlib/x.rs', 3), warning('j/src/b.rs', 4)].join('\n')
  assert.deepEqual(countClippy(out), {j: 1, k: 2})
  assert.deepEqual(auditClippy({j: 1, k: 2}, {j: 1, k: 2}), [])
  assert.match(auditClippy({k: 3}, {k: 2})[0], /k: 3 clippy warnings/)
  assert.match(auditClippy({k: 1}, {k: 2})[0], /below the pin/)
})

test('upward module references inside a layered crate are pinned', async () => {
  const {rootAliases, braceHeads} = await import('../scripts/moduleLayers.mjs')
  const text = 'use crate::{Result, curve::{Curve, find_span}, foundation::guards::Budget,\n surface::Surface};'
  assert.deepEqual(braceHeads(text, text.indexOf('{') + 1), ['curve', 'foundation', 'surface'])
  assert.deepEqual(rootAliases('pub use crate::core::curve;\npub use analysis::point_inversion as inversion;\n// pub use x::y;'), {curve: 'core', inversion: 'analysis'})
  const baseline = {sizes: {}, layers: [], moduleLayers: ['k: numerics -> core']}
  assert.deepEqual(auditArchitecture({sizes: {}, layers: [], moduleLayers: ['k: numerics -> core']}, baseline), [])
  assert.match(auditArchitecture({sizes: {}, layers: [], moduleLayers: ['k: numerics -> core', 'k: core -> sweeps']}, baseline)[0], /core -> sweeps: upward module dependency/)
})

test('baseline update keeps keys owned by other audits', () => {
  const baseline = {sizes: {}, layers: [], clippy: {k: 3}}
  assert.deepEqual(nextBaseline({sizes: {}, layers: []}, baseline).clippy, {k: 3})
})

test('clippy warnings in a file shared through #[path] count once, for the owning crate', async () => {
  const {countClippy} = await import('../scripts/audit-clippy.mjs')
  const warning = file => JSON.stringify({reason: 'compiler-message', message: {level: 'warning', message: 'unused', code: {code: 'dead_code'},
    spans: [{is_primary: true, file_name: file, line_start: 3, column_start: 1}]}})
  assert.deepEqual(countClippy([warning('k/examples/../../j/examples/support/a.rs'), warning('j/examples/support/a.rs')].join('\n')), {j: 1})
})

test('upward imports between src domains are pinned', async () => {
  const {SRC_LAYERS} = await import('../scripts/srcDomains.mjs')
  assert.ok(SRC_LAYERS.core < SRC_LAYERS.kernel && SRC_LAYERS.kernel < SRC_LAYERS.services && SRC_LAYERS.services < SRC_LAYERS.features)
  const baseline = {sizes: {}, layers: [], srcLayers: ['kernel -> services']}
  assert.deepEqual(auditArchitecture({sizes: {}, layers: [], srcLayers: ['kernel -> services']}, baseline), [])
  assert.match(auditArchitecture({sizes: {}, layers: [], srcLayers: ['kernel -> services', 'core -> kernel']}, baseline)[0], /core -> kernel: upward src domain import/)
})

test('module layer scan drops only inline test modules, including cfg(all(test, ...))', async () => {
  const {withoutTestModules} = await import('../scripts/moduleLayers.mjs')
  const text = 'a crate::x\n#[cfg(test)]\nmod tests { crate::y { } }\nb crate::z\n#[cfg(all(test, feature = "t"))]\nmod t2 { crate::w }\nc'
  const kept = withoutTestModules(text)
  assert.ok(kept.includes('crate::x') && kept.includes('crate::z') && kept.includes('c'))
  assert.ok(!kept.includes('crate::y') && !kept.includes('crate::w'))
})

test('a crate-root file declared with #[path] from a layer module belongs to that layer', async () => {
  const {mkdtempSync, mkdirSync, writeFileSync} = await import('node:fs')
  const {join} = await import('node:path')
  const {tmpdir} = await import('node:os')
  const {moduleOwners} = await import('../scripts/moduleLayers.mjs')
  const root = mkdtempSync(join(tmpdir(), 'owners-'))
  const src = 'crates/k/src/'
  mkdirSync(join(root, src, 'low'), {recursive: true})
  writeFileSync(join(root, src, 'low.rs'), 'mod inner;\n#[path = "helper.rs"]\nmod helper;\n#[cfg(test)]\n#[path = "probe.rs"]\nmod probe;\n')
  writeFileSync(join(root, src, 'low/inner.rs'), '')
  writeFileSync(join(root, src, 'helper.rs'), '')
  writeFileSync(join(root, src, 'probe.rs'), '')
  const files = ['low.rs', 'low/inner.rs', 'helper.rs', 'probe.rs'].map(f => src + f)
  const owner = moduleOwners(root, src, {low: 0, high: 1}, files)
  assert.equal(owner.get(src + 'helper.rs'), 'low')
  assert.equal(owner.get(src + 'low/inner.rs'), 'low')
  assert.equal(owner.get(src + 'probe.rs'), undefined)
})
