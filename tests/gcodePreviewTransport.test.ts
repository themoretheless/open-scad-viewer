import {expect,it} from 'vitest'
import {unpackGcodePreview,type PackedGcodePreview} from '../src/services/gcodePreviewTransport'

const packed=(moveRows:number[]):PackedGcodePreview=>({moveRows,layers:1,extrusionMm:1,depositedVolumeMm3:2,bounds:null,travelDistanceMm:0,printDistanceMm:10,estimatedTimeS:1})
it('preserves double precision, signed zero and metadata without retaining wire rows',()=>{
 const input=packed([-0,1.23456789012345,3,4,5,0,1])
 const result=unpackGcodePreview(input)
 expect(result.moves).toEqual([{x:-0,y:1.23456789012345,z:3,e:4,feedrateMmS:5,layerIndex:0,extruded:true}])
 expect(result).not.toHaveProperty('moveRows')
 input.moveRows[1]=99
 expect(result.moves[0].y).toBe(1.23456789012345)
 expect(unpackGcodePreview(packed([])).moves).toEqual([])
})
it('refuses incomplete, oversized, nonfinite and malformed rows',()=>{
 for(const rows of [[1],new Array(700_007).fill(0),[0,0,0,0,0,0,2],[0,0,0,0,0,-1,0],[0,0,0,0,0,2048,0],[0,0,0,0,0,1.5,0]])expect(()=>unpackGcodePreview(packed(rows))).toThrow('Invalid packed')
 for(let i=0;i<7;i++)for(const value of [NaN,Infinity,-Infinity]){
  const row=[0,0,0,0,0,0,0];row[i]=value
  expect(()=>unpackGcodePreview(packed(row))).toThrow('Invalid packed')
 }
 expect(()=>unpackGcodePreview(packed(null as unknown as number[]))).toThrow('Invalid packed')
})
