import {expect,it} from 'vitest'
import {checkLatticeGraphInput,isNominalLatticeGraph,type NominalLatticeGraph} from '../src/services/latticeGraphProtocol'
import {mainSolidResult} from '../src/services/mainSolidProtocol'
import type {LighteningOptions} from '../src/services/solidLightening'

const graph=():NominalLatticeGraph=>({modelKind:'nominal-bounding-box-axial',nodes:[[0,0,0],[1,0,0],[0,1,0],[0,0,1]],edges:[[0,1],[0,2],[0,3],[1,2],[1,3],[2,3]]})
it('admits only bounded, distinct three-dimensional nominal graphs',()=>{
  expect(mainSolidResult({kind:'latticeGraph'},graph())).toBe(true)
  for(const patch of [{modelKind:'part-strength'}, {nodes:[]}, {edges:[]}, {nodes:new Array(126)}, {edges:new Array(401)},
    {nodes:new Array(4)}, {edges:new Array(2)}, {edges:[[0,1],[1,0]]}, {edges:[[0,4]]}, {edges:[[0,0]]},
    {edges:[[0,.5]]}, {nodes:[[0,0,0],[1,0,0],[0,1,0],[1,1,0]]},
    {nodes:[[0,0,0],[1,0,0],[0,1,0],[0,1,0]]}, {nodes:[[0,0,0],[1,0,0],[0,1,0],[0,0,NaN]]}]) {
    expect(isNominalLatticeGraph({...graph(),...patch})).toBe(false)
  }
})
it('bounds cloned scene input before dispatch and never detaches it',()=>{
  const mesh={vertices:new Float32Array(18),indices:new Uint32Array([0,1,2]),transform:new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])}
  const options={pattern:'octet',cell:10,jitter:0,seed:42} as LighteningOptions
  expect(()=>checkLatticeGraphInput(mesh,options)).not.toThrow()
  expect(mesh.vertices.byteLength).toBe(72)
  for(const patch of [{vertices:new Float32Array(1)},{indices:new Uint32Array(300003)},
    {vertices:new Float32Array(4_194_306)},{transform:new Float32Array(15)}, {vertices:[]}]) {
    expect(()=>checkLatticeGraphInput({...mesh,...patch} as typeof mesh,options)).toThrow('Nominal lattice analysis')
  }
  expect(()=>checkLatticeGraphInput(mesh,{...options,pattern:'grid'})).toThrow('spatial lattice')
  for(const patch of [{cell:0},{cell:NaN},{jitter:-1},{seed:.5},{diagonals:'yes'}]) {
    expect(()=>checkLatticeGraphInput(mesh,{...options,...patch} as LighteningOptions)).toThrow('Spatial graph requires')
  }
  expect(()=>checkLatticeGraphInput({...mesh,transform:new Float32Array(16)},options)).toThrow('affine')
  for(const field of ['vertices','indices','transform'] as const){
    const original=mesh[field],Constructor=field==='indices'?Uint32Array:Float32Array
    for(const buffer of [new ArrayBuffer(16*1024*1024+64),new SharedArrayBuffer(original.byteLength)]){
      const view=new Constructor(buffer,0,original.length);view.set(original)
      expect(()=>checkLatticeGraphInput({...mesh,[field]:view},options)).toThrow('owned buffers')
    }
  }
  const sharedBacking=new ArrayBuffer(16*1024*1024)
  const aliases={vertices:new Float32Array(sharedBacking,0,18),indices:new Uint32Array(sharedBacking,72,3),transform:new Float32Array(sharedBacking,84,16)}
  aliases.transform.set(mesh.transform)
  expect(()=>checkLatticeGraphInput(aliases,options)).not.toThrow()
})
