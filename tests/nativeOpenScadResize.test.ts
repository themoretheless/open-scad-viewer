import {expect,it} from 'vitest'
import {languageRequest} from '../src/services/languages/kernel'
import {resolveOpenScadResize} from '../src/services/openScadStableGeometrySemantics'
import {parseOpenSCAD} from '../src/services/openscadParser'

it('preserves overflow ratios and ignores nonpositive targets before checking extents',()=>{
  expect(resolveOpenScadResize({dimension:2,newsize:[Number.MAX_VALUE,0],extents:[Number.MIN_VALUE,0],auto:true})).toEqual({scales:[Infinity,Infinity],applied:true,valid:true})
  const warnings:unknown[]=[]
  expect(resolveOpenScadResize({dimension:3,newsize:[0,4,5],extents:[NaN,0,0],auto:true},{warn:w=>warnings.push(w)})).toEqual({scales:[1,1,1],applied:false,valid:false})
  expect(warnings).toEqual([{code:'OPENSCAD_RESIZE_ZERO_EXTENT',message:'resize cannot map a zero-width axis 1 to a positive target',value:0}])
})

it('applies native aspect scales to real OpenSCAD solids',async()=>{
  for(const [auto,expected] of [['true',[10,6,4]],['[false,true,false]',[10,6,2]]] as const){
    const result=await parseOpenSCAD(`resize([10,0,0],auto=${auto}) cube([5,3,2]);`,{languageProfile:'openscad/stable-2021.01'})
    const max=[-Infinity,-Infinity,-Infinity]
    for(const mesh of result.meshes)for(let i=0;i<mesh.vertices.length;i+=6)for(let axis=0;axis<3;axis++)max[axis]=Math.max(max[axis],mesh.vertices[i+axis])
    expect(max).toEqual(expected)
  }
})

it('aggregates separated child bounds in Rust before resolving aspect axes',()=>{
  expect(resolveOpenScadResize({dimension:2,newsize:[20,0],auto:true,bounds:[
    {min:[5,-2],max:[7,1]}, {min:[-3,4],max:[0,6]},
  ]})).toEqual({scales:[2,2],applied:true,valid:true})
  const warnings:unknown[]=[]
  expect(resolveOpenScadResize({dimension:2,newsize:[2,0],bounds:[
    {min:[NaN,0],max:[4,3]},
  ]},{warn:w=>warnings.push(w)}).valid).toBe(false)
  expect(warnings).toEqual([{code:'OPENSCAD_RESIZE_ZERO_EXTENT',message:'resize cannot map a zero-width axis 0 to a positive target',value:NaN}])
})

it('keeps the nullable extents ABI accepted',()=>{
  expect(languageRequest(13,{targets:[2,null],extents:[null,3],automatic:[false,false]})).toMatchObject({
    ok:true,value:{scales:[1,1],invalidAxis:0},
  })
})
