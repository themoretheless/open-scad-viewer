import {expect,it} from 'vitest'
import {createBrepBox,prepareBrepDisplay} from '../src/services/geometry/brep'
it('prepares native display buffers with flat unit normals and mesh area',()=>{
 const display=prepareBrepDisplay(createBrepBox([0,0,0],[2,3,4]),1)
 expect(display).not.toHaveProperty('positions')
 expect(display).not.toHaveProperty('indices')
 expect(display.surfaceArea).toBe(52)
 expect(display.displayIndices).toEqual(Array.from({length:36},(_,index)=>index))
 expect(display.displayVertices).toHaveLength(216)
 expect(display.faceIds).toHaveLength(12)
 expect(new Set(display.faceIds).size).toBe(6)
 for(let offset=0;offset<display.displayVertices.length;offset+=18){
  const normal=display.displayVertices.slice(offset+3,offset+6)
  expect(Math.hypot(...normal)).toBe(1)
  expect(display.displayVertices.slice(offset+9,offset+12)).toEqual(normal)
  expect(display.displayVertices.slice(offset+15,offset+18)).toEqual(normal)
 }
})
