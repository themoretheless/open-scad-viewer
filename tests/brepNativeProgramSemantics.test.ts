import { describe, expect, it } from 'vitest'
import { executeBrepNativeProgram, type BrepNativeOutput } from '../src/services/brepNativeExecutor'
import { BrepSemanticBackendError } from '../src/services/brepSemanticErrors'
import { analyzeNurbsBrep, tessellateNurbsBrep } from '../src/services/geometry/brep'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'
import type { SemanticExecutionControl } from '../src/services/semanticProgramExecutor'

// Native-session successor of the retired per-node TypeScript BrepSemanticBackend
// suite: identical geometry/behavioral assertions, zero host lease state.
const source = (body: string) => `// @language openscad-viewer/brep-1\n${body}`
const lower = (body: string) => lowerOpenSCADToSemanticProgram(source(body))
const execute = (body: string, control: SemanticExecutionControl = {}) => executeBrepNativeProgram(lower(body), control)

function solid(output: BrepNativeOutput) {
  if (output.value.tag !== 'value' || output.value.geometry.kind !== 'solid') throw new Error('Expected a solid result')
  return output.value.geometry.model
}

describe('analytic B-rep native program execution', () => {
  it('executes exact primitive outputs with centered dimensions and preserves independent occurrence/color identity', async () => {
    const result = await execute('color("red") cube([2,4,6], center=true); sphere(r=3,$fn=5); cylinder(h=6,r1=2,r2=0,center=true); cylinder(h=4,r1=0,r2=2);')
    try {
      expect(result.outputs).toHaveLength(4)
      expect(new Set(result.outputs.map(output => output.occurrence.sceneEntityId)).size).toBe(4)
      expect(result.outputs[0].color).toEqual([1, 0, 0, 1])
      const [box, sphere, cone, invertedCone] = result.outputs.map(solid)
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
      const original = solid(result.outputs[0])
      const transformed = solid(result.outputs[1])
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
      const [box, ring] = result.outputs.map(solid)
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
    expect(empty.outputs.every(output => output.value.valueType.evidence.tag === 'representation-preserving')).toBe(true)
    await empty.dispose()
    const separate = await execute('union(){sphere(1);translate([5,0,0]) sphere(1);}')
    try { expect(solid(separate.outputs[0]).bodies).toHaveLength(2) }
    finally { await separate.dispose() }
  })

  it('keeps analytic topology independent of source display facet intent', async () => {
    const coarse = await execute('sphere(r=2,$fn=5);')
    const fine = await execute('sphere(r=2,$fn=120);')
    try {
      expect(solid(coarse.outputs[0])).toEqual(solid(fine.outputs[0]))
      expect(coarse.attestation.tessellationPolicyHash).not.toBe(fine.attestation.tessellationPolicyHash)
      expect(coarse.outputs[0].value.valueType.evidence).toEqual({ tag: 'representation-preserving' })
    } finally { await Promise.all([coarse.dispose(), fine.dispose()]) }
  })

  it('reports unsupported carriers and operations without using a mesh fallback or invalidating later executions', async () => {
    for (const body of ['offset(r=1) square(1);', 'hull(){cube(1);translate([3,0,0]) cube(1);}']) {
      await expect(execute(body)).rejects.toMatchObject({
        code: 'E_SEMANTIC_BACKEND_FAILURE',
        backendCause: { code: 'E_BREP_SEMANTIC_UNSUPPORTED' },
      })
    }
    await expect(execute('intersection(){sphere(2);translate([1,0,0]) sphere(2);}'))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_FAILURE', backendCause: { kernelCause: expect.any(Error) } })
    await expect(executeBrepNativeProgram(lowerOpenSCADToSemanticProgram('cube(1);')))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_BACKEND_BEGIN', backendCause: { code: 'E_BREP_SEMANTIC_UNSUPPORTED' } })
    const valid = await execute('cube(1);')
    await valid.dispose()
  })

  it('aborts between native graph steps and preserves unrelated committed snapshots', async () => {
    const first = await execute('cube(2);')
    const controller = new AbortController()
    await expect(execute('cube(3); cube(4);', {
      signal: controller.signal,
      onNode: () => controller.abort(),
    })).rejects.toMatchObject({ code: 'E_SEMANTIC_ABORTED' })
    expect(solid(first.outputs[0]).bodies).toHaveLength(1)
    await first.dispose()
    expect(first.disposed).toBe(true)
    await expect(executeBrepNativeProgram(structuredClone(lower('cube(1);'))))
      .rejects.toThrow(/minted/)
  })

  it('enforces the semantic node budget before any native node runs', async () => {
    await expect(execute('cube(1); cube(2);', { maxNodes: 1 }))
      .rejects.toMatchObject({ code: 'E_SEMANTIC_BUDGET' })
  })

  it('keeps exact profile carriers immutable and separate from the solid boundary', async () => {
    const result = await execute('square([2,4],center=true); circle(r=3,$fn=5);')
    const value = result.outputs[1].value
    if (value.tag !== 'value' || value.geometry.kind !== 'profile') throw new Error('Expected profile')
    const square = result.outputs[0].value
    if (square.tag !== 'value' || square.geometry.kind !== 'profile') throw new Error('Expected rectangle')
    expect(square.geometry.profile.areaMm2).toBeCloseTo(8, 9)
    const profile = value.geometry.profile
    expect(profile.areaMm2).toBeCloseTo(9 * Math.PI, 9)
    expect(profile.loops[0]).toHaveLength(4)
    expect(profile.loops[0].every(curve => curve.degree === 2)).toBe(true)
    expect(profile.loops[0][0].weights).toEqual([1, Math.SQRT1_2, 1])
    expect(Object.isFrozen(profile.loops[0][0].controlPoints[0])).toBe(true)
    expect(() => { profile.loops[0][0].controlPoints[0][0] = 99 }).toThrow(TypeError)
    await result.dispose()
  })

  it('extrudes exact centered circular and rectangular profiles without depending on display facet/slice intent', async () => {
    const result = await execute('linear_extrude(height=5,center=true,slices=7) circle(r=3,$fn=5); linear_extrude(height=6) square([2,4],center=true);')
    try {
      const [cylinder, box] = result.outputs.map(solid)
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
      const model = solid(result.outputs[0])
      expect(analyzeNurbsBrep(model).signedVolumeMm3).toBeCloseTo((100 - 36 + 4) * 3, 7)
      expect(model.bodies).toHaveLength(2)
      expect(model.faces.filter(face => face.holes.length > 0)).toHaveLength(2)
      expect(tessellateNurbsBrep(model, 2).report.closed).toBe(true)
    } finally { await result.dispose() }
  })

  it('retains circular Boolean trims through extrusion and handles Boolean profile emptiness', async () => {
    const result = await execute('linear_extrude(height=4) difference(){circle(r=3);circle(r=1);} linear_extrude(height=2) intersection(){circle(r=2);translate([1,0,0]) circle(r=2);} linear_extrude(height=3) difference(){circle(r=2);circle(r=2);}')
    try {
      const [ring, lens] = result.outputs.slice(0, 2).map(solid)
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
      const [ring, box] = result.outputs.map(solid)
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

it('derives the display policy hash natively with byte-identical host semantics',async()=>{
 const {callGeometryRust}=await import('../src/services/geometry/kernel')
 const {sha256Hex}=await import('../src/core/sha256')
 for(const policy of [{quality:'preview',segments:4},{quality:'full',segments:32},{quality:'preview',segments:1}]){
  const plan=callGeometryRust<{policy:string;displayPolicyHash:string}>('brep_scene_plan',policy)
  expect(plan.policy).toBe(JSON.stringify({version:1,sampling:'uniform-patch',...policy}))
  expect(plan.displayPolicyHash).toBe(sha256Hex('brep-display-policy-v1\n'+plan.policy))
 }
 for(const policy of [{quality:'draft',segments:4},{quality:'preview',segments:0},{quality:'preview',segments:33}]){
  expect(()=>callGeometryRust('brep_scene_plan',policy)).toThrow(/preview\/full quality and 1\.\.32 segments/)
 }
})
