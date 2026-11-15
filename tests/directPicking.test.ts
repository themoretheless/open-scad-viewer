import { expect, it } from 'vitest'
import { pickDirectMesh } from '../src/app/directPicking'

const square = (z: number) => ({ positions: [-1, -1, z, 1, -1, z, 1, 1, z, -1, 1, z], indices: [0, 1, 2, 0, 2, 3] })
it('picks the nearest triangle along the orthographic view ray', () => {
  const top = { yaw: 0, pitch: Math.PI / 2 }
  const hit = pickDirectMesh([0.5, 0.2], top, [
    { id: 'low', mesh: square(0) },
    { id: 'high', mesh: square(5) },
  ])
  expect(hit?.id).toBe('high')
  expect(pickDirectMesh([-0.5, 0.2], top, [{ id: 'low', mesh: square(0) }])?.id).toBe('low')
  expect(pickDirectMesh([3, 3], top, [{ id: 'low', mesh: square(0) }])).toBeNull()
})
