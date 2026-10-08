import { readFileSync, existsSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const catalog = JSON.parse(readFileSync(resolve(root, 'docs/design/nurbs-catalog.json'), 'utf8'))
const errors = []
const ids = new Set(), counts = {}
const statuses = new Set(['planned', 'native', 'integrated', 'qualified'])
for (const figure of catalog.figures) {
  if (!/^NURBS-\d{4}$/.test(figure.id) || ids.has(figure.id)) errors.push(`Invalid or duplicate ID: ${figure.id}`)
  ids.add(figure.id)
  if (!figure.name?.trim()) errors.push(`${figure.id}: missing name`)
  if (!statuses.has(figure.status)) errors.push(`${figure.id}: unknown status`)
  counts[figure.status] = (counts[figure.status] ?? 0) + 1
  for (const file of figure.evidence ?? []) {
    if (!existsSync(resolve(root, file))) errors.push(`${figure.id}: missing evidence ${file}`)
  }
  if (figure.status !== 'planned') {
    const symbol = figure.nativeApi?.split('::').at(-1)
    if (!symbol || !(figure.evidence ?? []).some(file => file.endsWith('.rs') && existsSync(resolve(root,file)) && new RegExp(`pub\\s+fn\\s+${symbol}(?:\\s*\\(|\\s*<)`).test(readFileSync(resolve(root,file),'utf8')))) {
      errors.push(`${figure.id}: native API not found in source evidence`)
    }
  }
  if (figure.status === 'qualified') {
    // Evidence paths alone never prove geometry or integration qualification.
    for (const gate of ['native', 'json', 'wasm', 'typescript', 'rush', 'editor', 'geometry', 'export']) {
      const proof = figure.qualification?.[gate]
      if (!proof?.artifact || !proof?.command || proof.passed !== true || !existsSync(resolve(root,proof.artifact))) errors.push(`${figure.id}: missing ${gate} qualification`)
    }
    if (figure.remainingQualification?.length) errors.push(`${figure.id}: qualified with remaining work`)
  }
}
if (catalog.specifiedCount !== catalog.figures.length) errors.push('specifiedCount differs from figure count')
if (!Number.isInteger(catalog.targetCount) || catalog.targetCount < catalog.specifiedCount) errors.push('Invalid targetCount')
if (catalog.unallocatedCount !== catalog.targetCount - catalog.specifiedCount) errors.push('Incorrect unallocatedCount')
if (process.argv.includes('--complete') && (catalog.specifiedCount !== catalog.targetCount || counts.qualified !== catalog.targetCount)) errors.push('Target is not fully specified and qualified')
console.log(JSON.stringify({ target: catalog.targetCount, specified: catalog.specifiedCount, unallocated: catalog.unallocatedCount, counts, errors }, null, 2))
if (errors.length) process.exitCode = 1
