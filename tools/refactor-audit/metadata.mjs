// cargo metadata -> roots.tsv (crate, target kind, root file) and deps.tsv (crate, extern ident, kind, package, optional)
// usage: node metadata.mjs <metadata.json> <out_dir>
import {readFileSync, writeFileSync} from 'node:fs'
const [metaPath, out] = process.argv.slice(2)
const m = JSON.parse(readFileSync(metaPath, 'utf8'))
const members = new Set(m.workspace_members), roots = [], deps = []
const KIND = {lib: 'lib', rlib: 'lib', cdylib: 'lib', 'proc-macro': 'lib', bin: 'bin', test: 'test', bench: 'bench', example: 'example', 'custom-build': 'build'}
for (const p of m.packages) {
  if (!members.has(p.id)) continue
  const dir = p.manifest_path.replace(/\/Cargo\.toml$/, '').split('/').pop()
  for (const t of p.targets) roots.push([dir, KIND[t.kind[0]] ?? t.kind[0], t.src_path].join('\t'))
  const lib = p.targets.find(t => t.kind.some(k => k === 'lib' || k === 'rlib' || k === 'cdylib'))
  deps.push([dir, (lib?.name ?? p.name).replace(/-/g, '_'), 'self'].join('\t'))
  for (const d of p.dependencies) deps.push([dir, (d.rename ?? d.name).replace(/-/g, '_'), d.kind ?? 'normal', d.name, d.optional ? 'optional' : ''].join('\t'))
}
writeFileSync(`${out}/roots.tsv`, roots.join('\n') + '\n')
writeFileSync(`${out}/deps.tsv`, deps.join('\n') + '\n')
console.log(`roots=${roots.length} deps=${deps.length}`)
