import { expect, it } from 'vitest'
import { geometryTransformTransition } from '../src/services/geometryTransformTransition'
import { identity, rotateZ, scale, translate } from '../src/services/math3d'

it('rotates rigidly through 180 degrees while interpolating translation and scale', () => {
  const from = identity()
  const to = translate(rotateZ(scale(identity(), [3,3,3]), Math.PI), [10,0,0])
  const transition = geometryTransformTransition(from, to)!
  const mid = transition(0.5)
  expect(mid[0]).toBeCloseTo(0)
  expect(Math.abs(mid[4])).toBeCloseTo(2)
  expect(mid[3]).toBeCloseTo(5)
  expect(transition(0)).toEqual(from)
  expect(transition(1)).toEqual(to)
})
it('rejects singular, sheared and changing-reflection matrices for dissolve', () => {
  const shear = identity(); shear[1] = 0.5
  expect(geometryTransformTransition(identity(), shear)).toBeNull()
  expect(geometryTransformTransition(identity(), scale(identity(), [0,1,1]))).toBeNull()
  expect(geometryTransformTransition(identity(), scale(identity(), [-1,1,1]))).toBeNull()
})
