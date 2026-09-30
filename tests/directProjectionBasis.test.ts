import {expect,it} from 'vitest'
import {createDirectProjector,projectDirectPoint,directFaceShade} from '../src/services/directModelingTools'
it('matches point and normal projection exactly across camera directions and coordinate scales',()=>{
 for(const yaw of [-3.14,-1,0,1,3.14])for(const pitch of [-1.5,-.1,0,.1,1.5]){
  const camera={yaw,pitch},project=createDirectProjector(camera)
  for(const p of [[0,0,0],[1,2],[1e6,-1e6,1e-8],[-7,3,9],[.00001,2,1]])expect(project(p)).toEqual(projectDirectPoint(p,camera))
  const mesh={positions:new Float64Array([0,0,0,2,3,1,-4,1,2]),indices:new Uint32Array([0,1,2])}
  expect(directFaceShade(mesh,0,camera,project)).toBe(directFaceShade(mesh,0,camera))
 }
})
it('captures a camera snapshot instead of mixing bases when the caller updates its camera',()=>{
 const camera={yaw:1,pitch:.5},project=createDirectProjector(camera),expected=projectDirectPoint([2,3,4],camera)
 camera.yaw=2;camera.pitch=-.5
 expect(project([2,3,4])).toEqual(expected)
 expect(project([2,3,4])).not.toEqual(projectDirectPoint([2,3,4],camera))
})
