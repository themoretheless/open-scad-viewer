import {describe,expect,it} from 'vitest'
import {brepSourceSnapshot} from '../src/services/geometry/brep/sourceSnapshot'
import type {NurbsBrep} from '../src/services/geometry/brep'

describe('B-rep wire identity',()=>{
 const model:NurbsBrep={vertices:[],edges:[],loops:[],faces:[],shells:[],bodies:[],toleranceMm:1e-7,topologyIds:{vertices:[],edges:[],loops:[],faces:[],shells:[],bodies:[]}}
 it('equates absent serde metadata with explicit empty defaults',()=>{
  expect(brepSourceSnapshot(model)).toBe(brepSourceSnapshot({...model,topologyIds:{...model.topologyIds!,localTolerancesMm:{},sewedEdges:[]}}))
 })
 it('retains local tolerances, sewn identities and authored geometry',()=>{
  const base=brepSourceSnapshot(model)
  expect(brepSourceSnapshot({...model,topologyIds:{...model.topologyIds!,localTolerancesMm:{edge:1e-5}}})).not.toBe(base)
  expect(brepSourceSnapshot({...model,topologyIds:{...model.topologyIds!,sewedEdges:['e:1']}})).not.toBe(base)
  expect(brepSourceSnapshot({...model,vertices:[{point:[1,2,3]}]})).not.toBe(base)
 })
})
