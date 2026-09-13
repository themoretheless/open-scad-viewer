import { describe, expect, it } from 'vitest'
import type { SemanticNode } from '../src/core/semanticProgram'
import {
  BREP_SEMANTIC_CARRIERS,
  BrepSemanticBackendError,
  createBrepSemanticBackend,
  inspectBrepSemanticPayload,
  inspectBrepSemanticProfilePayload,
  type BrepSemanticPayloadFamily,
  type BrepSemanticBackendOptions,
} from '../src/services/brepSemanticBackend'
import { analyzeNurbsBrep, createBrepBox, tessellateNurbsBrep } from '../src/services/geometry/brep'
import { MAX_NATIVE_GEOMETRY_CHARACTERS } from '../src/core/nativeGeometry'
import {
  executeSemanticProgram,
  type SemanticBackendContext,
  type SemanticBackendEvaluation,
  type SemanticBackendPayloadLease,
  type SemanticRuntimeValue,
} from '../src/services/semanticProgramExecutor'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const source = (body: string) => `// @language openscad-viewer/brep-1\n${body}`
const lower = (body: string) => lowerOpenSCADToSemanticProgram(source(body))
const execute = (body: string) => executeSemanticProgram(lower(body), createBrepSemanticBackend())

function payload(value: SemanticRuntimeValue<BrepSemanticPayloadFamily>) {
  if (value.tag !== 'value') throw new Error('Expected a solid result')
  return inspectBrepSemanticPayload(value.payload)
}

function sessionFixture(body = 'cube([2,3,4]); translate([5,6,7]) cube([1,2,3]);', options: BrepSemanticBackendOptions = {}) {
  const lowering = lower(body)
  const signal = new AbortController().signal
  const begin = {
    programHash: lowering.attestation.programHash,
    languageContract: 'openscad-viewer/brep-1' as const,
    limits: { maxNodes: lowering.program.core.nodes.length },
    signal,
  }
  const session = createBrepSemanticBackend(options).begin(begin)
  const context = (node: SemanticNode, inputNodeIndices: readonly number[] = []): SemanticBackendContext => ({
    nodeIndex: node.id,
    carrierKey: `${node.valueType.geometryKind}/${node.valueType.space}/${node.valueType.representation}`,
    inputNodeIndices,
    producer: null,
    programHash: begin.programHash,
    languageContract: begin.languageContract,
    signal,
  })
  return { session, context, nodes: lowering.program.core.nodes, begin }
}

function nonEmpty(value: SemanticBackendEvaluation<BrepSemanticPayloadFamily>) {
  if (value.tag !== 'value') throw new Error('Expected a leased solid result')
  return value
}

describe('analytic B-rep semantic backend', () => {
  it('executes exact primitive outputs with centered dimensions and preserves independent occurrence/color identity', async () => {
    expect(BREP_SEMANTIC_CARRIERS).toEqual(['solid/d3/analytic-brep', 'solid-set/d3/analytic-brep', 'region/d2/analytic-brep'])
    const result = await execute('color("red") cube([2,4,6], center=true); sphere(r=3,$fn=5); cylinder(h=6,r1=2,r2=0,center=true); cylinder(h=4,r1=0,r2=2);')
    try {
      expect(result.outputs).toHaveLength(4)
      expect(new Set(result.outputs.map(output => output.occurrence.sceneEntityId)).size).toBe(4)
      expect(result.outputs[0].color).toEqual([1, 0, 0, 1])
      const [box, sphere, cone, invertedCone] = result.outputs.map(output => payload(output.value).model)
      expect(box.vertices.map(vertex => vertex.point[2]).sort((a, b) => a - b)).toEqual([-3, -3, -3, -3, 3, 3, 3, 3])
      expect(analyzeNurbsBrep(box).signedVolumeMm3).toBeCloseTo(48, 8)
      expect(sphere.faces.every(face => face.surface.degreeU === 2 && face.surface.degreeV === 2)).toBe(true)
      expect(sphere.edges.some(edge => edge.curve.weights.some(weight => weight !== 1))).toBe(true)
      expect(Math.min(...cone.vertices.map(vertex => vertex.point[2]))).toBe(-3)
      expect(Math.max(...cone.vertices.map(vertex => vertex.point[2]))).toBe(3)
      expect(Math.min(...invertedCone.vertices.map(vertex => vertex.point[2]))).toBe(0)
      expect(Math.max(...invertedCone.vertices.map(vertex => vertex.point[2]))).toBe(4)
      for (const model of [box, sphere, cone, invertedCone]) {
        expect(Object.isFrozen(model)).toBe(true)
        expect(Object.isFrozen(model.faces[0].surface.controlPoints[0][0])).toBe(true)
        expect(tessellateNurbsBrep(model, 4).report.closed).toBe(true)
      }
    } finally { await result.dispose() }
  })

  it('maps column-major affine transforms and reflections onto authored carriers without mutating primitive inputs', async () => {
    const result = await execute('cube([2,3,4]); multmatrix([[-2,1,0,10],[0,3,0,-4],[0,0,0.5,7],[0,0,0,1]]) cube([2,3,4]);')
    try {
      const original = payload(result.outputs[0].value).model
      const transformed = payload(result.outputs[1].value).model
      expect(transformed.vertices.map(vertex => vertex.point)).toEqual(original.vertices.map(({ point: [x, y, z] }) => [-2 * x + y + 10, 3 * y - 4, .5 * z + 7]))
      expect(analyzeNurbsBrep(original).signedVolumeMm3).toBeCloseTo(24, 8)
      expect(analyzeNurbsBrep(transformed).signedVolumeMm3).toBeCloseTo(72, 8)
      expect(tessellateNurbsBrep(transformed, 2).report.closed).toBe(true)
      expect(() => { original.vertices[0].point[0] = 99 }).toThrow(TypeError)
      expect(original.vertices[0].point[0]).toBe(0)
    } finally { await result.dispose() }
  })

  it('folds native planar and circular-prismatic Boolean operands in authored order', async () => {
    const result = await execute('difference(){cube([10,4,4]); cube([2,4,4]); translate([8,0,0]) cube([2,4,4]);} difference(){cylinder(r=3,h=5); cylinder(r=1,h=5);}')
    try {
      const [box, ring] = result.outputs.map(output => payload(output.value).model)
      expect(analyzeNurbsBrep(box).signedVolumeMm3).toBeCloseTo(96, 7)
      expect(analyzeNurbsBrep(ring).signedVolumeMm3).toBeCloseTo(40 * Math.PI, 6)
      expect(ring.edges.some(edge => edge.curve.degree === 2)).toBe(true)
      expect(ring.faces.filter(face => face.holes.length > 0)).toHaveLength(2)
      expect(tessellateNurbsBrep(ring, 4).report.closed).toBe(true)
    } finally { await result.dispose() }
  })

  it('keeps canonical empty results typed and disposes an execution with no geometry', async () => {
    const none = await execute('')
    expect(none.outputs).toEqual([])
    await Promise.all([none.dispose(), none.dispose()])
    expect(none.disposed).toBe(true)
    const empty = await execute('translate([7,8,9]) difference(){cube(2);cube(2);} intersection(){sphere(1);translate([5,0,0]) sphere(1);}')
    expect(empty.outputs.map(output => output.value.tag)).toEqual(['empty', 'empty'])
    expect(empty.outputs.every(output => !('payload' in output.value))).toBe(true)
    await empty.dispose()
    const separate = await execute('union(){sphere(1);translate([5,0,0]) sphere(1);}')
    try { expect(payload(separate.outputs[0].value).model.bodies).toHaveLength(2) }
    finally { await separate.dispose() }
  })

  it('keeps analytic topology independent of source display facet intent', async () => {
    const coarse = await execute('sphere(r=2,$fn=5);')
    const fine = await execute('sphere(r=2,$fn=120);')
    try {
      expect(payload(coarse.outputs[0].value).model).toEqual(payload(fine.outputs[0].value).model)
      expect(coarse.attestation.tessellationPolicyHash).not.toBe(fine.attestation.tessellationPolicyHash)
      expect(coarse.outputs[0].value.evidence).toEqual({ tag: 'representation-preserving' })
    } finally { await Promise.all([coarse.dispose(), fine.dispose()]) }
  })

  it('reports unsupported carriers and operations without using a mesh fallback or invalidating later sessions', async () => {
    const backend = createBrepSemanticBackend()
    for (const body of ['offset(r=1) square(1);', 'hull(){cube(1);translate([3,0,0]) cube(1);}']) {
      await expect(executeSemanticProgram(lower(body), backend)).rejects.toMatchObject({
        code: 'E_SEMANTIC_BACKEND_FAILURE',
        backendCause: { code: 'E_BREP_SEMANTIC_UNSUPPORTED' },
      })
    }
    await expect(executeSemanticProgram(lower('intersection(){sphere(2);translate([1,0,0]) sphere(2);}'), backend))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', backendCause: { kernelCause: expect.any(Error) } })
    await expect(executeSemanticProgram(lowerOpenSCADToSemanticProgram('cube(1);'), backend))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_BEGIN', backendCause: { code: 'E_BREP_SEMANTIC_UNSUPPORTED' } })
    const valid = await executeSemanticProgram(lower('cube(1);'), backend)
    await valid.dispose()
  })

  it('issues unique leases, releases intermediates on commit, and rejects inspection after result disposal', async () => {
    const { session, context, nodes } = sessionFixture()
    const first = nonEmpty(await session.evaluate(nodes[0], [], context(nodes[0])))
    const second = nonEmpty(await session.evaluate(nodes[1], [], context(nodes[1])))
    expect(first.lease).not.toBe(second.lease)
    expect(session.validatePayload(first.payload.carrierKey, first.payload, first.lease)).toBe(true)
    const closing = session.close({ tag: 'commit', retained: [second.lease] })
    expect(session.close({ tag: 'abort', code: 'E_SEMANTIC_ABORTED', node: null })).toBe(closing)
    const closed = await closing
    expect(closed.tag).toBe('committed')
    expect(() => inspectBrepSemanticPayload(first.payload)).toThrow(/released/)
    expect(inspectBrepSemanticPayload(second.payload).model.bodies).toHaveLength(1)
    expect(() => session.evaluate(nodes[0], [], context(nodes[0]))).toThrow(/closed/)
    await expect(session.releasePayload(second.lease)).rejects.toMatchObject({ code: 'E_BREP_SEMANTIC_LIFECYCLE' })
    if (closed.tag !== 'committed') throw new Error('Expected committed lease')
    const disposed = closed.resultLease.dispose()
    expect(closed.resultLease.dispose()).toBe(disposed)
    await disposed
    expect(() => inspectBrepSemanticPayload(second.payload)).toThrow(/released/)
  })

  it('rejects foreign, released, and swapped payloads across simultaneous sessions', async () => {
    const a = sessionFixture('translate([1,2,3]) cube(2);')
    const b = sessionFixture('translate([1,2,3]) cube(2);')
    const value = nonEmpty(await a.session.evaluate(a.nodes[0], [], a.context(a.nodes[0])))
    expect(b.session.validatePayload(value.payload.carrierKey, value.payload, value.lease)).toBe(false)
    await expect(b.session.releasePayload(value.lease)).rejects.toMatchObject({ code: 'E_BREP_SEMANTIC_CONTRACT' })
    expect(() => b.session.evaluate(b.nodes[1], [value], b.context(b.nodes[1], [0]))).toThrow(/foreign/)
    await b.session.close({ tag: 'failure', code: 'E_SEMANTIC_BACKEND_FAILURE', node: 1 })
    await Promise.all([a.session.releasePayload(value.lease), a.session.releasePayload(value.lease)])
    expect(a.session.validatePayload(value.payload.carrierKey, value.payload, value.lease)).toBe(false)
    expect(() => a.session.evaluate(a.nodes[1], [value], a.context(a.nodes[1], [0]))).toThrow(/stale/)
    await a.session.close({ tag: 'abort', code: 'E_SEMANTIC_ABORTED', node: 1 })
    expect(() => inspectBrepSemanticPayload({ ...value.payload })).toThrow(BrepSemanticBackendError)
  })

  it('closes all resources when commit retains a duplicate, foreign, or already released lease', async () => {
    for (const kind of ['duplicate', 'foreign', 'released']) {
      const { session, context, nodes } = sessionFixture('cube(1);')
      const value = nonEmpty(await session.evaluate(nodes[0], [], context(nodes[0])))
      if (kind === 'released') await session.releasePayload(value.lease)
      const retained = kind === 'duplicate' ? [value.lease, value.lease]
        : kind === 'foreign' ? [Object.freeze({}) as SemanticBackendPayloadLease] : [value.lease]
      await expect(session.close({ tag: 'commit', retained })).rejects.toMatchObject({ code: 'E_BREP_SEMANTIC_CONTRACT' })
      expect(() => inspectBrepSemanticPayload(value.payload)).toThrow(/released/)
    }
  })

  it('rejects mismatched execution context and cancellation and preserves unrelated committed snapshots', async () => {
    const { session, context, nodes } = sessionFixture('cube(1);')
    expect(() => session.evaluate(nodes[0], [], { ...context(nodes[0]), programHash: 'foreign' })).toThrow(/execution context/)
    await session.close({ tag: 'failure', code: 'E_SEMANTIC_BACKEND_FAILURE', node: 0 })
    const first = await execute('cube(2);')
    const firstPayload = first.outputs[0].value
    const controller = new AbortController()
    await expect(executeSemanticProgram(lower('cube(3); cube(4);'), createBrepSemanticBackend(), {
      signal: controller.signal,
      onNode: () => controller.abort(),
    })).rejects.toMatchObject({ code: 'E_SEMANTIC_ABORTED' })
    expect(payload(firstPayload).model.bodies).toHaveLength(1)
    await first.dispose()
    expect(() => payload(firstPayload)).toThrow(/released/)
    await expect(executeSemanticProgram(structuredClone(lower('cube(1);')), createBrepSemanticBackend()))
      .rejects.toThrow(/minted/)
  })

  it('keeps exact profile carriers immutable and separate from the solid inspection boundary', async () => {
    const result = await execute('square([2,4],center=true); circle(r=3,$fn=5);')
    const value = result.outputs[1].value
    if (value.tag !== 'value') throw new Error('Expected profile')
    const square = result.outputs[0].value
    if (square.tag !== 'value') throw new Error('Expected rectangle')
    expect(inspectBrepSemanticProfilePayload(square.payload).profile.areaMm2).toBeCloseTo(8, 9)
    const profile = inspectBrepSemanticProfilePayload(value.payload).profile
    expect(profile.areaMm2).toBeCloseTo(9 * Math.PI, 9)
    expect(profile.loops[0]).toHaveLength(4)
    expect(profile.loops[0].every(curve => curve.degree === 2)).toBe(true)
    expect(profile.loops[0][0].weights).toEqual([1, Math.SQRT1_2, 1])
    expect(Object.isFrozen(profile.loops[0][0].controlPoints[0])).toBe(true)
    expect(() => inspectBrepSemanticPayload(value.payload)).toThrow(/requires extrusion/)
    expect(() => { profile.loops[0][0].controlPoints[0][0] = 99 }).toThrow(TypeError)
    await result.dispose()
    expect(() => inspectBrepSemanticProfilePayload(value.payload)).toThrow(/released/)
  })

  it('extrudes exact centered circular and rectangular profiles without depending on display facet/slice intent', async () => {
    const result = await execute('linear_extrude(height=5,center=true,slices=7) circle(r=3,$fn=5); linear_extrude(height=6) square([2,4],center=true);')
    try {
      const [cylinder, box] = result.outputs.map(output => payload(output.value).model)
      expect(analyzeNurbsBrep(cylinder).signedVolumeMm3).toBeCloseTo(45 * Math.PI, 6)
      expect(Math.min(...cylinder.vertices.map(vertex => vertex.point[2]))).toBe(-2.5)
      expect(Math.max(...cylinder.vertices.map(vertex => vertex.point[2]))).toBe(2.5)
      expect(cylinder.edges.some(edge => edge.curve.degree === 2)).toBe(true)
      expect(tessellateNurbsBrep(cylinder, 4).report.closed).toBe(true)
      expect(analyzeNurbsBrep(box).signedVolumeMm3).toBeCloseTo(48, 8)
    } finally { await result.dispose() }
  })

  it('orients unordered even-odd polygon rings with holes and islands before extrusion', async () => {
    const result = await execute('linear_extrude(height=3) polygon(points=[[2,2],[8,2],[8,8],[2,8],[0,0],[0,10],[10,10],[10,0],[4,4],[4,6],[6,6],[6,4]], paths=[[0,1,2,3],[4,5,6,7],[8,9,10,11]]);')
    try {
      const model = payload(result.outputs[0].value).model
      expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo((100 - 36 + 4) * 3, 7)
      expect(model.bodies).toHaveLength(2)
      expect(model.faces.filter(face => face.holes.length > 0)).toHaveLength(2)
      expect(tessellateNurbsBrep(model, 2).report.closed).toBe(true)
    } finally { await result.dispose() }
  })

  it('retains circular Boolean trims through extrusion and handles Boolean profile emptiness', async () => {
    const result = await execute('linear_extrude(height=4) difference(){circle(r=3);circle(r=1);} linear_extrude(height=2) intersection(){circle(r=2);translate([1,0,0]) circle(r=2);} linear_extrude(height=3) difference(){circle(r=2);circle(r=2);}')
    try {
      const [ring, lens] = result.outputs.slice(0, 2).map(output => payload(output.value).model)
      expect(analyzeNurbsBrep(ring).signedVolumeMm3).toBeCloseTo(32 * Math.PI, 6)
      expect(analyzeNurbsBrep(lens).signedVolumeMm3).toBeCloseTo((8 * Math.acos(.25) - .5 * Math.sqrt(15)) * 2, 6)
      expect(lens.edges.some(edge => edge.curve.degree === 2)).toBe(true)
      expect(tessellateNurbsBrep(lens, 4).report.closed).toBe(true)
      expect(result.outputs[2].value.tag).toBe('empty')
    } finally { await result.dispose() }
  })

  it('supports profile translations, rotations, uniform scaling, reflection, and nonuniform polygon scaling', async () => {
    const result = await execute('linear_extrude(height=2) translate([7,-3]) rotate(37) mirror([1,2]) scale(2) difference(){circle(r=3);circle(r=1);} linear_extrude(height=3) scale([2,3]) polygon([[0,0],[2,0],[2,1],[0,1],[0,0]]);')
    try {
      const [ring, box] = result.outputs.map(output => payload(output.value).model)
      expect(analyzeNurbsBrep(ring).signedVolumeMm3).toBeCloseTo(64 * Math.PI, 5)
      expect(tessellateNurbsBrep(ring, 4).report.closed).toBe(true)
      expect(analyzeNurbsBrep(box).signedVolumeMm3).toBeCloseTo(36, 8)
    } finally { await result.dispose() }
  })

  it('refuses unsupported elliptical profiles, out-of-plane transforms, twisted/tapered extrusion, and crossing rings', async () => {
    for (const body of [
      'linear_extrude(height=2) scale([2,1]) circle(2);',
      'linear_extrude(height=2) scale([1.0000000001,1]) circle(2);',
      'linear_extrude(height=2) translate([0,0,1]) square(2);',
      'linear_extrude(height=2,twist=30) square(2);',
      'linear_extrude(height=2,scale=[1,2]) circle(2);',
    ]) {
      await expect(execute(body)).rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', backendCause: { code: 'E_BREP_SEMANTIC_UNSUPPORTED' } })
    }
    await expect(execute('linear_extrude(height=2) polygon([[0,0],[2,2],[0,2],[2,0]]);'))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', backendCause: expect.any(BrepSemanticBackendError) })
  })

  it('applies the same session lease ownership rules to profiles before extrusion', async () => {
    const a = sessionFixture('linear_extrude(height=3) circle(2);')
    const b = sessionFixture('linear_extrude(height=3) circle(2);')
    const profile = nonEmpty(await a.session.evaluate(a.nodes[0], [], a.context(a.nodes[0])))
    expect(a.session.validatePayload('region/d2/analytic-brep', profile.payload, profile.lease)).toBe(true)
    expect(a.session.validatePayload('solid/d3/analytic-brep', profile.payload, profile.lease)).toBe(false)
    expect(() => b.session.evaluate(b.nodes[1], [profile], b.context(b.nodes[1], [0]))).toThrow(/foreign/)
    await b.session.close({ tag: 'failure', code: 'E_SEMANTIC_BACKEND_FAILURE', node: 1 })
    const solid = nonEmpty(await a.session.evaluate(a.nodes[1], [profile], a.context(a.nodes[1], [0])))
    expect(() => inspectBrepSemanticProfilePayload(solid.payload)).toThrow(/not a planar profile/)
    const committed = await a.session.close({ tag: 'commit', retained: [solid.lease] })
    expect(() => inspectBrepSemanticProfilePayload(profile.payload)).toThrow(/released/)
    expect(inspectBrepSemanticPayload(solid.payload).model.bodies).toHaveLength(1)
    if (committed.tag !== 'committed') throw new Error('Expected commit')
    await committed.resultLease.dispose()
    expect(() => inspectBrepSemanticPayload(solid.payload)).toThrow(/released/)
  })

  it('bounds active geometry snapshots before publishing and restores capacity only when leases are released', async () => {
    const geometryCharacters = JSON.stringify(createBrepBox([0, 0, 0], [1, 1, 1])).length
    const { session, context, nodes } = sessionFixture('cube(1);cube(1);', { maxRetainedGeometryCharacters: geometryCharacters })
    expect(nodes).toHaveLength(2)
    const first = nonEmpty(await session.evaluate(nodes[0], [], context(nodes[0])))
    await Promise.all([session.releasePayload(first.lease), session.releasePayload(first.lease)])
    const second = nonEmpty(await session.evaluate(nodes[1], [], context(nodes[1])))
    const committed = await session.close({ tag: 'commit', retained: [second.lease] })
    if (committed.tag !== 'committed') throw new Error('Expected commit')
    expect(inspectBrepSemanticPayload(second.payload).model.bodies).toHaveLength(1)
    await committed.resultLease.dispose()

    const full = sessionFixture('cube(1);cube(1);', { maxRetainedGeometryCharacters: 2 * geometryCharacters - 1 })
    const retained = nonEmpty(await full.session.evaluate(full.nodes[0], [], full.context(full.nodes[0])))
    expect(() => full.session.evaluate(full.nodes[1], [], full.context(full.nodes[1])))
      .toThrowError(expect.objectContaining({ code: 'E_BREP_SEMANTIC_BUDGET' }))
    await full.session.close({ tag: 'failure', code: 'E_SEMANTIC_BACKEND_FAILURE', node: 1 })
    expect(() => inspectBrepSemanticPayload(retained.payload)).toThrow(/released/)
    await expect(executeSemanticProgram(lower('cube(1);'), createBrepSemanticBackend({ maxRetainedGeometryCharacters: geometryCharacters - 1 })))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', backendCause: { code: 'E_BREP_SEMANTIC_BUDGET' } })
    for (const maxRetainedGeometryCharacters of [0, -1, .5, Infinity, MAX_NATIVE_GEOMETRY_CHARACTERS + 1]) {
      expect(() => createBrepSemanticBackend({ maxRetainedGeometryCharacters })).toThrow(/limit must be/)
    }
  })
})

it('enforces reduced-node arity and result space inside the Rust endpoint',async()=>{
 const {callGeometryRust}=await import('../src/services/geometry/kernel')
 const node={kind:'box',valueType:{space:'d3'},size:[1,1,1],center:false}
 expect(()=>callGeometryRust('brep_semantic_geometry',{node,inputs:[{kind:'solid'}]})).toThrow(/cannot consume operands/)
 expect(()=>callGeometryRust('brep_semantic_geometry',{node:{...node,valueType:{space:'d2'}},inputs:[]})).toThrow(/result space/)
 expect(()=>callGeometryRust('brep_semantic_geometry',{node:{kind:'boolean',operation:'union'},inputs:[]})).toThrow(/explicit d2 or d3/)
})

it('rejects unknown singleton Boolean operations in Rust for profiles and solids',async()=>{
 const {callGeometryRust}=await import('../src/services/geometry/kernel')
 for(const space of ['d2','d3']){
  const node=space==='d2'?{kind:'circle-analytic',valueType:{space},radius:2}:{kind:'box',valueType:{space},size:[1,1,1],center:false}
  const operand=callGeometryRust('brep_semantic_geometry',{node,inputs:[]})
  expect(()=>callGeometryRust('brep_semantic_geometry',{node:{kind:'boolean',valueType:{space},operation:'not-a-boolean'},inputs:[operand]})).toThrow(/Unsupported semantic Boolean/)
  expect(callGeometryRust('brep_semantic_geometry',{node:{kind:'boolean',valueType:{space},operation:'union'},inputs:[operand]})).toEqual(operand)
 }
})
