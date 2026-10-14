import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {runGlobalGeometryChecks} from './globalGeometryChecks'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
const path=resolve(process.argv[2]??'docs/qualification/global-geometry-2026-10-14/nonplanar-fixtures.json.gz')
const output=resolve(process.argv[3]??'/tmp/osv-global-wasm-qualification.json')
const bytes=readFileSync(path),fixtures=JSON.parse((path.endsWith('.gz')?gunzipSync(bytes):bytes).toString('utf8'))
const report=runGlobalGeometryChecks(fixtures,message=>console.log(message))
mkdirSync(resolve(output,'..'),{recursive:true})
writeFileSync(output,JSON.stringify({...report,fixturesSha256:createHash('sha256').update(bytes).digest('hex'),artifact:sweepStepArtifactProvenance(),runtime:process.version},null,2)+'\n')
console.log(`Passed ${report.passed} cases, ${report.refused} typed refusals, five intersection pair kinds`)
