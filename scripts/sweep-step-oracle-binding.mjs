import assert from 'node:assert/strict'
import {createReadStream} from 'node:fs'
import {createHash} from 'node:crypto'

// Metadata recapture may reuse an independent oracle only for identical STEP
// bytes. Every consumer still verifies every current file against stepSha256.
export async function bindStepOracle(manifest,manifestSha256,oracle){
 assert.equal(oracle.passed,true)
 assert.deepEqual(oracle.artifactProvenance,manifest.artifactProvenance)
 const qualified=process.argv.find(a=>a.startsWith('--qualified-manifest='))?.slice('--qualified-manifest='.length)
 if(oracle.manifestSha256===manifestSha256)return {method:'same-manifest',oracleManifestSha256:oracle.manifestSha256}
 assert.ok(qualified,'Oracle belongs to another manifest; an explicit qualified source receipt is required')
 const hash=createHash('sha256')
 for await(const chunk of createReadStream(qualified))hash.update(chunk)
 assert.equal(hash.digest('hex'),oracle.manifestSha256,'Original qualified manifest does not bind this oracle')
 assert.equal(manifest.cases.length,oracle.cases.length,'Incomplete exact STEP identity inventory')
 assert.equal(new Set(manifest.cases.map(c=>c.file)).size,manifest.cases.length,'Duplicate recaptured STEP identity')
 assert.equal(new Set(oracle.cases.map(c=>c.file)).size,oracle.cases.length,'Duplicate qualified STEP identity')
 const imported=new Map(oracle.cases.map(c=>[c.file,c]))
 for(const item of manifest.cases){
  assert.equal(imported.get(item.file)?.stepSha256,item.sha256,'Recaptured STEP differs from the independently imported artifact')
 }
 return {method:'exact-step-identity-from-qualified-manifest',oracleManifestSha256:oracle.manifestSha256,
  currentManifestSha256:manifestSha256,scope:'Reuses the original independent import of identical STEP bytes; not a new OpenCascade run.'}
}
