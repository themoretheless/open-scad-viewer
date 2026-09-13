import {readFileSync, statSync} from 'node:fs'
import {resolve} from 'node:path'
import {createBrepDiagnosticSupervisor} from '../src/mcp/brepDiagnosticSupervisor.ts'
import {BREP_DIAGNOSTIC_LIMITS} from '../src/services/brepDiagnosticProtocol.ts'

// Internal diagnostic entry point. It does not register or impersonate a provider.
const [file, rawSegments = '6', ...extra] = process.argv.slice(2)
if (!file || extra.length || !/^(?:[1-9]|[12][0-9]|3[0-2])$/.test(rawSegments)) {
  console.error('Usage: node --import tsx scripts/check-brep-semantic.mjs source.scad [segments:1..32]')
  process.exitCode = 2
} else {
  const controller = new AbortController()
  const cancel = () => controller.abort()
  process.once('SIGINT', cancel)
  try {
    const path = resolve(file)
    const stat = statSync(path)
    if (!stat.isFile() || stat.size > BREP_DIAGNOSTIC_LIMITS.sourceCharacters * 4) {
      throw new RangeError('Expected a bounded UTF-8 source file')
    }
    const source = new TextDecoder('utf-8', {fatal:true}).decode(readFileSync(path))
    const lane = createBrepDiagnosticSupervisor()
    const receipt = await lane.evaluate(source, {quality:'full', segments:Number(rawSegments)}, {signal:controller.signal})
    console.log(JSON.stringify({
      sourceFile:path,
      identity:receipt.identity,
      attestation:receipt.scene.attestation,
      displayPolicyHash:receipt.displayPolicyHash,
      outputs:receipt.scene.outputs,
      metrics:{status:receipt.scene.metrics, deviation:receipt.scene.deviationStatus,
        triangles:receipt.scene.result.meshes.reduce((n, mesh) => n + mesh.indices.length / 3, 0),
        volumeMm3:receipt.scene.result.volume, surfaceAreaMm2:receipt.scene.result.surfaceArea},
      worker:lane.snapshot(),
    }, null, 2))
  } catch (error) {
    console.error(JSON.stringify({code:error?.code ?? null, message:error instanceof Error ? error.message : String(error)}))
    process.exitCode = controller.signal.aborted ? 130 : 1
  } finally {
    process.removeListener('SIGINT', cancel)
  }
}
