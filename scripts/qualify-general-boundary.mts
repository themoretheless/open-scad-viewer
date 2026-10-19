import {readFileSync, writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {callGeometryRust, warmGeometryKernel} from '../src/services/geometry/kernel'
import {runGeneralBoundaryChecks} from './generalBoundaryChecks.mjs'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
const path = process.argv[2] ?? 'crates/geometry-bridge/tests/fixtures/general-frame-cap-certificate-v1.json'
const output = process.argv[3] ?? '/tmp/osv-general-boundary-wasm.json'
const bytes = readFileSync(path)
await warmGeometryKernel()
const report = runGeneralBoundaryChecks(JSON.parse(bytes.toString('utf8')), request => {
  const {op, ...args} = request
  return callGeometryRust(op, args)
})
writeFileSync(output, JSON.stringify({...report, runtime: process.version,
  fixtureSha256: createHash('sha256').update(bytes).digest('hex'), artifact: sweepStepArtifactProvenance()}, null, 2) + '\n')
console.log(`General boundary: ${report.passed} checks passed`)
