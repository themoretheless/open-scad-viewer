import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const read = p => readFileSync(resolve(root, p), 'utf8')
const domains = JSON.parse(read('docs/design/nurbs-api-domains.json'))
const library = read('crates/nurbs-core/src/lib.rs').replace(/pub use ([\w:]+)::\{([^{}]+)\};/g,
  (_, prefix, entries) => entries.split(',').map(entry => entry.trim()).filter(Boolean)
    .map(entry => `pub use ${prefix}::${entry};`).join('\n'))
// The catalog describes the compatibility API. Structural parents own the
// moved modules; their public module re-exports retain the original flat API.
const exports = [...library.matchAll(/^pub use ((?:crate::)?[\w:]+)(?: as (\w+))?;/gm)]
const moduleExports = exports.filter(([_, path]) => {
  const source = path.replace(/^crate::/, '').replaceAll('::', '/')
  return existsSync(resolve(root, 'crates/nurbs-core/src', source + '.rs')) ||
    existsSync(resolve(root, 'crates/nurbs-core/src', source, 'mod.rs'))
})
const parents = new Set(moduleExports.map(([_, path]) => path.replace(/^crate::/, '').split('::')[0]))
const catalogModules = new Set(Object.values(domains).flat())
const modules = [...new Set([
  ...[...library.matchAll(/^pub mod (\w+);/gm)].map(x => x[1]).filter(x => x !== 'domains' && (!parents.has(x) || catalogModules.has(x))),
  ...moduleExports.map(([_, path, alias]) => alias ?? path.split('::').at(-1)),
])]

const owner = new Map()
for (const [domain, names] of Object.entries(domains)) {
  for (const name of names) {
    if (owner.has(name)) throw new Error(`Duplicate domain assignment: ${name}`)
    if (!modules.includes(name)) throw new Error(`Unknown module: ${name}`)
    owner.set(name, domain)
  }
}
const unassigned = modules.filter(name => !owner.has(name))
if (unassigned.length) throw new Error(`Unassigned public modules: ${unassigned.join(', ')}`)
const facade = read('crates/nurbs-core/src/domains.rs')
for (const [domain, names] of Object.entries(domains)) {
  const block = facade.match(new RegExp(`pub mod ${domain} \\{([^}]+)\\}`))?.[1]
  for (const name of names) if (!block?.includes(`pub use crate::${name};`)) throw new Error(`Missing re-export: ${domain}::${name}`)
}
const catalog = JSON.parse(read('docs/design/nurbs-catalog.json'))
const cell = x => String(x ?? '—').replaceAll('|', '\\|').replaceAll('\n', ' ')
const lines = ['# Карта NURBS API', '', 'Генерируется командой `node scripts/map-nurbs-api.mjs`. Проверка актуальности: та же команда с `--check`.', '', 'Статусы, evidence и оставшаяся квалификация перенесены из каталога; наличие evidence не означает новый запуск тестов. `integrated` не означает квалификацию всех адаптеров или STEP. Ограничения алгоритмов описаны в документации соответствующего API; эта карта их не расширяет.', '', '| ID | Возможность | Раздел | API | Статус | Evidence | Оставшаяся квалификация |', '|---|---|---|---|---|---|---|']
for (const f of catalog.figures) {
  const parts = f.nativeApi?.split('::') ?? []
  const domain = parts[0] === 'nurbs_core' ? owner.get(parts[1]) : parts[0] === 'brep_core' ? 'brep-core' : 'planned'
  if (parts[0] === 'nurbs_core' && !domain) throw new Error(`Unmapped API: ${f.nativeApi}`)
  lines.push('| ' + [f.id, f.name, domain, f.nativeApi, f.status, (f.evidence ?? []).join('; '), (f.remainingQualification ?? []).join('; ')].map(cell).join(' | ') + ' |')
}
const path = 'docs/design/nurbs-api-map.md'
const output = lines.join('\n') + '\n'
if (process.argv.includes('--check')) {
  if (read(path) !== output) throw new Error('API map is stale; run node scripts/map-nurbs-api.mjs')
} else writeFileSync(resolve(root, path), output)
console.log(`Mapped ${modules.length} public modules and ${catalog.figures.length} catalog entries`)
