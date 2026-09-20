import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {fileURLToPath} from 'node:url'
import {
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS, GEOMETRY_MANIFEST_ARCHIVE,
  computeGeometryManifestDigest,
  type GeometryEngineStaticManifest, type GeometryExecutionDescriptor,
} from '../src/core/geometryExecution'

const sha256 = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex')
const identityFields = ['engineClass', 'engineKey', 'kernelFingerprint',
  'semanticProgramVersion', 'capabilityManifestVersion', 'manifestDigest'] as const

export function assessRuntimeBinding(
  manifest: GeometryEngineStaticManifest,
  execution: GeometryExecutionDescriptor,
  observedKernelSha256: string,
) {
  assert.match(observedKernelSha256, /^[a-f0-9]{64}$/)
  const manifestIntegrityMatches = computeGeometryManifestDigest(manifest) === manifest.manifestDigest
  const descriptorMatchesManifest = identityFields.every(field => execution[field] === manifest[field])
  const observedKernelFingerprint = `sha256:${observedKernelSha256}`
  const artifactMatchesManifest = observedKernelFingerprint === manifest.kernelFingerprint
  const runtimeEvidence = execution.evidence === 'runtime'
  return {
    engineClass: manifest.engineClass,
    manifestVersion: manifest.capabilityManifestVersion,
    declaredQualification: manifest.qualification.status,
    declaredKernelFingerprint: manifest.kernelFingerprint,
    observedKernelFingerprint,
    manifestIntegrityMatches, descriptorMatchesManifest, artifactMatchesManifest, runtimeEvidence,
    matches: manifestIntegrityMatches && descriptorMatchesManifest && artifactMatchesManifest && runtimeEvidence,
  }
}

/** A fresh-process smoke audit, never a qualification run or evidence publisher. */
export async function auditRuntimeManifests() {
  const {isGeometryKernelReady} = await import('../src/services/geometry/kernel')
  assert.equal(isGeometryKernelReady(), false, 'Runtime audit requires a fresh geometry realm')
  const {default: packed} = await import('../src/generated/geometry-kernels/bytes')
  const {unpackBrotliWasmBase64} = await import('../src/services/wasmBrotliPacking')
  const {setOptionalWasmCompiler} = await import('../src/services/wasmCompilation')
  const {compileWasmArtifact} = await import('../src/services/wasmArtifact')
  const {GeometryBuildEngine} = await import('../src/services/geometryBuildEngine')
  const artifact = unpackBrotliWasmBase64(packed)
  const artifactSha256 = sha256(artifact)
  const publicArtifact = readFileSync(new URL('../public/wasm/geometry-kernel.wasm', import.meta.url))
  let compilerLoads = 0
  setOptionalWasmCompiler(async (url, identity) => {
    if (url !== '/wasm/geometry-kernel.wasm') return null
    compilerLoads++
    assert.ok(identity)
    return compileWasmArtifact(artifact, identity)
  })
  try {
    const engine = new GeometryBuildEngine()
    const request = {quality: 'full', purpose: 'analysis'} as const
    const records = []
    for (const [engineClass, source] of [
      ['mesh', 'cube(1);'],
      ['brep', '// @language openscad-viewer/brep-1\ncube(1);'],
    ] as const) {
      await engine.initializeSource(source, request)
      const {execution, result} = await engine.buildSource(source, request)
      assert.ok(result.meshes.length > 0, `${engineClass} produced no geometry`)
      assert.ok(Number.isFinite(result.volume) && result.volume > 0)
      const manifest = GEOMETRY_MANIFEST_ARCHIVE[ACTIVE_GEOMETRY_MANIFEST_VERSIONS[engineClass]]
      records.push({...assessRuntimeBinding(manifest, execution, artifactSha256),
        meshCount: result.meshes.length, volumeMm3: result.volume})
    }
    assert.equal(compilerLoads, 1, 'Probe must execute the selected artifact in one shared kernel')
    const publicArtifactMatches = Buffer.from(artifact).equals(publicArtifact)
    return {
      schema: 1, qualificationClaim: 'none',
      scope: 'Fresh Node host, two cube builds using the measured embedded geometry artifact; not browser or full qualification',
      artifactBytes: artifact.byteLength, artifactSha256,
      publicArtifactSha256: sha256(publicArtifact), publicArtifactMatches, compilerLoads, records,
      matches: publicArtifactMatches && records.every(record => record.matches),
    }
  } finally {
    setOptionalWasmCompiler(undefined)
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const report = await auditRuntimeManifests()
  console.log(JSON.stringify(report, null, 2))
  if (!report.matches) process.exitCode = 1
}
