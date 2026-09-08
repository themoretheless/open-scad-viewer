import {describe,it,expect} from 'vitest'
import {boundedSceneEntityId} from '../src/core/boundedSceneEntityId'
describe('bounded scene identities',()=>{
 it('preserves short ids and deterministically bounds distinct deep instance paths',()=>{
  expect(boundedSceneEntityId('root')).toBe('entity:root')
  const path='root>op:module'.repeat(80)
  expect(boundedSceneEntityId(path).length).toBeLessThanOrEqual(256)
  expect(boundedSceneEntityId(path)).toBe(boundedSceneEntityId(path))
  expect(boundedSceneEntityId(path+'>loop:1')).not.toBe(boundedSceneEntityId(path+'>loop:2'))
 })
})
