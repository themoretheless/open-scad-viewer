import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {fileURLToPath} from 'node:url'
import {
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS,
  GEOMETRY_MANIFEST_ARCHIVE,
  type GeometryEngineStaticManifest,
} from '../src/core/geometryExecution'

type RuntimeBinding = Pick<GeometryEngineStaticManifest,
  'engineClass' | 'capabilityManifestVersion' | 'kernelFingerprint' | 'manifestDigest' | 'qualification'>

/** Diagnose actual active bindings, not only the newer pending catalog entries. */
export function compareRuntimeGeometryIdentity(
  kernel: Uint8Array,
  bindings: readonly RuntimeBinding[] = Object.values(ACTIVE_GEOMETRY_MANIFEST_VERSIONS)
    .map(version => GEOMETRY_MANIFEST_ARCHIVE[version]),
) {
  if (!bindings.length) throw new Error('No runtime bindings to audit')
  const observedFingerprint = `sha256:${createHash('sha256').update(kernel).digest('hex')}`
  const records = bindings.map(binding => ({
    engineClass: binding.engineClass,
    capabilityManifestVersion: binding.capabilityManifestVersion,
    manifestDigest: binding.manifestDigest,
    qualificationStatus: binding.qualification.status,
    expectedFingerprint: binding.kernelFingerprint,
    observedFingerprint,
    matches: binding.kernelFingerprint === observedFingerprint,
  }))
  return {schema: 1, qualificationClaim: 'none', kernelBytes: kernel.byteLength,
    matches: records.every(record => record.matches), records}
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  // Both registered providers currently execute this shared WASM artifact.
  const kernelPath = 'src/generated/geometry-kernels/kernel_bg.wasm'
  const root = fileURLToPath(new URL('../', import.meta.url))
  const report = compareRuntimeGeometryIdentity(readFileSync(resolve(root, kernelPath)))
  console.log(JSON.stringify({kernelPath, ...report}, null, 2))
  if (!report.matches) process.exitCode = 1
}
