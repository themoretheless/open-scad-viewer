import { expect, it } from 'vitest'
import { createBrepCylinder, tessellateNurbsBrep } from '../src/services/geometry/brep'
import { pushPullFace, solidTopology } from '../src/services/directSolidTools'
import { inspectPolygonMesh } from '../src/services/geometry/polygon'

it('pushes a displayed cylinder cap through the retained B-rep editor', () => {
  const brep = createBrepCylinder(3, 5)
  const body = { id: 'cylinder', name: 'Cylinder', brep, mesh: tessellateNurbsBrep(brep, 1) }
  const before = JSON.stringify(body)
  const top = solidTopology(body.mesh).faces.findIndex(f => f.normal[2] > .99)
  expect(top).toBeGreaterThanOrEqual(0)
  const result = pushPullFace(body, top, 20)
  expect(result.brep).toBeDefined()
  expect(result.brep!.faces).toHaveLength(6)
  const original = inspectPolygonMesh(body.mesh)
  const edited = inspectPolygonMesh(result.mesh)
  expect(edited.closed).toBe(true)
  expect(edited.degenerateTriangles).toBe(0)
  expect(edited.signedVolumeMm3 / original.signedVolumeMm3).toBeCloseTo(5, 5)
  expect(JSON.stringify(body)).toBe(before)
})
