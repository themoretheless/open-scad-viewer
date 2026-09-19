import { describe, expect, it } from 'vitest'
import type { CadKernelHandle, CadKernelOps } from '../src/services/cadKernelOps'
import { createBrepRecordingKernelOps } from '../src/services/solid/brepRecorder'
import { modelGraphNurbsSchema } from '../src/services/modelGraphNurbs'

/**
 * The recorder only needs distinct handle identities from the polygon track, so the
 * base is a stub. What is under test is the exact graph recorded beside it.
 */
function stubKernel(): CadKernelOps {
  const handle = () => ({ dimension: 3 } as unknown as CadKernelHandle)
  const stub = new Proxy({} as CadKernelOps, {
    get(_target, property) {
      if (property === 'implementationKey') return 'own-rust-cad-plan-v1'
      if (property === 'isEmpty') return () => false
      if (property === 'bounds') return () => ({ min: [0, 0, 0], max: [1, 1, 1] })
      if (property === 'delete') return () => {}
      return () => handle()
    },
  })
  return stub
}

function documentOf(nodes: readonly unknown[], root: string) {
  return { language: 'modelgraph/nurbs-1', units: 'mm', parameters: [], nodes, root }
}

describe('B-rep recording kernel', () => {
  it('records a centred cube, a cylinder and their difference as an exact graph', () => {
    const { ops, recording } = createBrepRecordingKernelOps(stubKernel())
    const cube = ops.box([10, 10, 10], true)
    const pin = ops.cylinder(12, 3, 3, 32, false)
    const cut = ops.boolean3('difference', [cube, pin])

    const resolved = recording.resolve(cut)
    expect(resolved).toHaveProperty('id')
    const parsed = modelGraphNurbsSchema.parse(
      documentOf(recording.nodes, (resolved as { id: string }).id),
    )
    const ops_ = parsed.nodes.map(node => node.op)
    expect(ops_).toEqual(['brep_box', 'brep_cylinder', 'brep_boolean'])
    const box = parsed.nodes[0] as { min: number[]; max: number[] }
    expect(box.min).toEqual([-5, -5, -5])
    expect(box.max).toEqual([5, 5, 5])
  })

  it('shifts a centred cylinder, because both kernels place the base at z = 0', () => {
    const { ops, recording } = createBrepRecordingKernelOps(stubKernel())
    const centred = ops.cylinder(10, 2, 2, 32, true)
    const resolved = recording.resolve(centred) as { id: string }
    const parsed = modelGraphNurbsSchema.parse(documentOf(recording.nodes, resolved.id))
    expect(parsed.nodes.map(node => node.op)).toEqual(['brep_cylinder', 'transform'])
    const shift = parsed.nodes[1] as { matrix: number[][] }
    expect(shift.matrix[2][3]).toBe(-5)
  })

  it('extrudes a recorded circle profile through exact arcs', () => {
    const { ops, recording } = createBrepRecordingKernelOps(stubKernel())
    const disc = ops.circle(4, 32)
    const solid = ops.linearExtrude(disc, 6, 1, 0, [1, 1], false)
    const resolved = recording.resolve(solid) as { id: string }
    const parsed = modelGraphNurbsSchema.parse(documentOf(recording.nodes, resolved.id))
    const curves = parsed.nodes.filter(node => node.op === 'curve')
    expect(curves).toHaveLength(4)
    // Exactness of a quadratic arc rests on the cos(45 degrees) shoulder weight.
    expect((curves[0] as { weights: number[] }).weights[1]).toBeCloseTo(Math.SQRT1_2, 12)
    const extrude = parsed.nodes.at(-1) as { op: string; loops: string[][]; z_max: number }
    expect(extrude.op).toBe('brep_extrude_curves')
    expect(extrude.loops[0]).toHaveLength(4)
    expect(extrude.z_max).toBe(6)
  })

  it('names the operation that made a body inexact instead of approximating it', () => {
    const { ops, recording } = createBrepRecordingKernelOps(stubKernel())
    const hulled = ops.hull3([ops.box([1, 1, 1], false)])
    const moved = ops.translate(hulled, [1, 2, 3])
    expect(recording.resolve(moved)).toEqual({ inexact: 'hull' })
  })

  it('refuses a twisted extrusion, which is no longer a straight prism', () => {
    const { ops, recording } = createBrepRecordingKernelOps(stubKernel())
    const twisted = ops.linearExtrude(ops.rectangle([4, 4], true), 10, 8, 45, [1, 1], false)
    expect(recording.resolve(twisted)).toEqual({ inexact: 'linear_extrude with twist' })
  })
})
