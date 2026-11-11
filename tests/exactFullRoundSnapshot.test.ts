import { beforeAll, describe, expect, it, vi } from 'vitest'

// Optional qualification-only module delivery substitution. Without an external
// module these tests use the normal installed package. Every geometry call still
// executes the real Rust ABI; production delivery and worker bootstrap are not
// replaced or claimed by this test.
vi.mock('../src/kernel/geometry/kernelCompilation', async importOriginal => {
  const path = process.env.OWN_EXACT_FULL_ROUND_WASM
  if (!path) return importOriginal()
  const { readFileSync } = await import('node:fs')
  const { createHash } = await import('node:crypto')
  const receipt = JSON.parse(readFileSync(new URL('../docs/qualification/full-round-display-wasm-2026-11-02/results.json', import.meta.url),'utf8'))
  const bytes = readFileSync(path)
  if (createHash('sha256').update(bytes).digest('hex') !== receipt.wasm_sha256) throw new Error('Unqualified exact WASM bytes')
  const module = new WebAssembly.Module(bytes)
  return { compileGeometryKernelArtifactSync: () => module, compileGeometryKernelArtifact: async () => module }
})
import { callGeometryRust } from '../src/kernel/geometry/kernel'
import { lowerOpenSCADToSemanticProgram } from '../src/language/semantic/semanticProgramLowerer'
import { createExactFullRoundSnapshot, restoreExactFullRoundSnapshot } from '../src/kernel/exactFullRoundSnapshot'
import { createNativeGeometryArtifact, isNativeGeometryArtifact, nativeFaceReference, assertNativeReferenceCurrent } from '../src/core/nativeGeometry'
import { prepareExactFullRoundDisplay } from '../src/kernel/geometry/brepExactDisplay'

function construct(height: number): { artifact: any; program: any } {
  const source = '// @language openscad-viewer/brep-1\nfull_round_exact([5,4,3]) cube([4,8,'+height+']);'
  const program = lowerOpenSCADToSemanticProgram(source).program
  const output: any = callGeometryRust('brep_graph', { ...program.core, op: 'brep_graph', sourceText: source,
    envelope: program, source: program.source, provenance: program.provenance,
    tessellationIntents: program.tessellationIntents, diagnostics: program.diagnostics,
    maxNodes: 4, maxBytes: 4194304 })
  return { artifact: output.outcomes[0].geometry.artifact, program }
}
describe('exact Full Round snapshot with qualified real WASM', () => {
  let artifact: any, program: any
  beforeAll(() => { ({artifact,program}=construct(6)) }, 30000)
  it('preserves exact coefficients, IDs and proof recipe through storage roundtrip', () => {
    const snapshot = createExactFullRoundSnapshot('body', artifact, program)
    expect(snapshot.kind).toBe('rational-brep')
    const transported = JSON.parse(JSON.stringify(snapshot))
    expect(isNativeGeometryArtifact(transported)).toBe(true)
    const restored = restoreExactFullRoundSnapshot(transported)
    expect(restored.artifact).toEqual(artifact)
    expect(restored.model).toEqual(artifact.geometry)
    expect(typeof (restored.model as any).vertices[0].point[0][0]).toBe('string')
    expect(Object.isFrozen(restored.artifact.geometry)).toBe(true)
    expect(Object.isFrozen((restored.model as any).vertices[0].point[0])).toBe(true)
  }, 30000)
  it('keeps geometry revision independent of display sampling and document revision', () => {
    const first = createExactFullRoundSnapshot('body', artifact, program)
    const low = prepareExactFullRoundDisplay(artifact,2), high = prepareExactFullRoundDisplay(artifact,16)
    expect(high.displayIndices.length).toBeGreaterThan(low.displayIndices.length)
    const second = createExactFullRoundSnapshot('body', artifact, { changedDocument:true })
    expect(second.revision).toBe(first.revision)
    expect(second.documentRevision).not.toBe(first.documentRevision)
    expect(() => assertNativeReferenceCurrent(second,nativeFaceReference(first,0))).not.toThrow()
  }, 30000)
  it('rejects geometric forgery even with valid freshly recomputed storage hashes', () => {
    const forged = structuredClone(artifact)
    forged.geometry.vertices[0].point[0] = ['999','1']
    const snapshot = createNativeGeometryArtifact('body','rational-brep',{encoding:'exact-full-round-snapshot/1',artifact:forged},program)
    expect(isNativeGeometryArtifact(snapshot)).toBe(true)
    expect(() => restoreExactFullRoundSnapshot(snapshot)).toThrow(/differs/)
    expect(() => createExactFullRoundSnapshot('body',forged,program)).toThrow(/differs/)
  }, 30000)
  it('rejects stale selections after an actual geometric edit', () => {
    const first = createExactFullRoundSnapshot('body',artifact,program)
    const edited = construct(7)
    const second = createExactFullRoundSnapshot('body',edited.artifact,edited.program)
    expect(second.revision).not.toBe(first.revision)
    expect(() => assertNativeReferenceCurrent(second,nativeFaceReference(first,0))).toThrow(/stale/)
  }, 30000)
  it('refuses damaged transport, wrong carrier and unknown payload fields', () => {
    const snapshot=createExactFullRoundSnapshot('body',artifact,program)
    expect(() => restoreExactFullRoundSnapshot({...snapshot,geometryJson:'{}'})).toThrow()
    const accessor = { ...snapshot }
    Object.defineProperty(accessor,'geometryJson',{enumerable:true,get(){throw new Error('getter executed')}})
    expect(() => restoreExactFullRoundSnapshot(accessor)).toThrow(/Invalid exact native/)
    const wrong=createNativeGeometryArtifact('body','brep',{encoding:'exact-full-round-snapshot/1',artifact},program)
    expect(() => restoreExactFullRoundSnapshot(wrong)).toThrow(/exact rational/)
    const extra=createNativeGeometryArtifact('body','rational-brep',{encoding:'exact-full-round-snapshot/1',artifact,extra:true},program)
    expect(() => restoreExactFullRoundSnapshot(extra)).toThrow(/payload/)
  }, 30000)
})
