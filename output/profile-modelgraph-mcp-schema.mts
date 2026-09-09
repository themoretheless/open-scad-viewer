/** Run with: node --import tsx output/profile-modelgraph-mcp-schema.mts */
import { readFileSync, writeFileSync } from 'node:fs'
import { performance } from 'node:perf_hooks'
import { cpus } from 'node:os'
import { isDeepStrictEqual } from 'node:util'
import { modelGraphSchema as before } from './modelGraph-runtime-reference'
import { modelGraphSchema as after } from '../src/services/modelGraph'
const document = JSON.parse(readFileSync('output/modelgraph-schema-profile-skadis.json', 'utf8'))
if (!isDeepStrictEqual(before.parse(document), after.parse(document))) throw new Error('Normalized document differs')
const runs = [['recursive_union', before], ['discriminant_dispatch', after]] as const
const raw: Record<string, number[]> = {}
for (const [name, schema] of runs) { raw[name] = []; for (let i = 0; i < 5; i++) schema.parse(document) }
for (let batch = 0; batch < 6; batch++) {
  for (const [name, schema] of batch % 2 ? [...runs].reverse() : runs) {
    const start = performance.now()
    for (let i = 0; i < 10; i++) schema.parse(document)
    raw[name]!.push((performance.now() - start) / 10)
  }
}
const median = (values: number[]) => { const sorted = [...values].sort((a, b) => a - b); return (sorted[2]! + sorted[3]!) / 2 }
const report = {
  node: process.version, cpu: cpus()[0]?.model,
  method: 'Same SKADIS graph; Node Zod schema only; 5 warmups, 6 alternating batches of 10; per-run milliseconds',
  median_ms: Object.fromEntries(Object.entries(raw).map(([name, values]) => [name, median(values)])), raw,
}
writeFileSync('output/modelgraph-mcp-schema-profile.json', JSON.stringify(report, null, 2) + '\n')
console.log(report)
