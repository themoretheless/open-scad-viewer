import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {fileURLToPath} from 'node:url'
import {FROZEN_ARCHIVES, REFRESH_OUTPUTS} from './refresh-qualification-fingerprints.mjs'

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')

/** Read-only diagnostics: never refresh bindings or create qualification evidence. */
export function auditQualificationDrift(root, {
  fingerprintPath = REFRESH_OUTPUTS.fingerprint,
  evidencePath = 'docs/qualification/own-rust-cad-v11.json',
  archives = FROZEN_ARCHIVES,
} = {}) {
  const read = path => {
    try { return readFileSync(resolve(root, path)) }
    catch (error) { if (error.code === 'ENOENT') return null; throw error }
  }
  const readDocument = path => {
    const bytes = read(path)
    if (bytes === null) throw new Error(`Missing required evidence document: ${path}`)
    return JSON.parse(bytes)
  }
  const fingerprint = readDocument(fingerprintPath)
  const evidence = readDocument(evidencePath)
  const records = []
  const compare = (kind, path, expectedSha256, expectedBytes, bytes = read(path)) => {
    const observedSha256 = bytes === null ? null : sha256(bytes)
    records.push({kind, path, expectedSha256, observedSha256,
      expectedBytes: expectedBytes ?? null, observedBytes: bytes?.length ?? null,
      status: bytes === null ? 'missing' : observedSha256 === expectedSha256
        && (expectedBytes === undefined || expectedBytes === bytes.length) ? 'match' : 'mismatch'})
  }
  for (const [path, hash] of Object.entries(archives)) compare('historical-archive', path, hash)
  for (const artifact of fingerprint.artifacts) compare('frozen-input', artifact.path, artifact.sha256, artifact.byteLength)
  compare('kernel', evidence.wasm.path, evidence.wasm.sha256, evidence.wasm.byteLength)
  for (const [path, field] of [
    ['THIRD_PARTY_NOTICES.md', 'noticesSha256'],
    ['package-lock.json', 'npmLockfileSha256'],
    ['crates/Cargo.lock', 'rustLockfileSha256'],
  ]) compare('dependency', path, evidence.dependencyFingerprints[field])
  const sourceParts = []
  const missingSources = []
  for (const path of evidence.sourceBundle.paths) {
    const bytes = read(path)
    if (bytes === null) missingSources.push(path)
    else sourceParts.push(Buffer.from(path), Buffer.from([0]), bytes, Buffer.from('\n'))
  }
  compare('source-bundle', evidencePath, evidence.sourceBundle.sha256, undefined,
    missingSources.length ? null : Buffer.concat(sourceParts))
  return {schema: 1, qualificationClaim: 'none', fingerprintPath, evidencePath,
    matches: records.every(record => record.status === 'match'), missingSources, records}
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const report = auditQualificationDrift(fileURLToPath(new URL('../', import.meta.url)))
  console.log(JSON.stringify(report, null, 2))
  if (!report.matches) process.exitCode = 1
}
