import {readFileSync} from 'node:fs'
import {describe,expect,it} from 'vitest'
import {compareRuntimeGeometryIdentity} from '../scripts/audit-runtime-geometry-identity.mts'

describe('actual packaged runtime provider binding',()=>{
 it('binds both active production providers to the actual packaged WASM',()=>{
  const bytes=readFileSync(new URL('../src/generated/geometry-kernels/kernel_bg.wasm',import.meta.url))
  const report=compareRuntimeGeometryIdentity(bytes)
  expect(report.records).toHaveLength(2)
  expect(report.records.map(record=>record.engineClass).sort()).toEqual(['brep','mesh'])
  expect(report.records.every(record=>record.qualificationStatus==='qualified')).toBe(true)
  expect(report.matches).toBe(true)
 })
})
