import {expect,it} from 'vitest'
import {decodeMeshBytesInKernel,renderTriangleSoupInKernel} from '../src/services/geometry/meshImportKernel'

it('copies detached import buffers before releasing their native handle',()=>{
 const bytes=new TextEncoder().encode('v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n')
 const first=decodeMeshBytesInKernel('obj',bytes)
 for(let i=0;i<10;i++)decodeMeshBytesInKernel('obj',bytes)
 expect(first.positions).toEqual(Float64Array.from([0,0,0,1,0,0,0,1,0]))
 expect(first.indices).toEqual(Uint32Array.from([0,1,2]))
})
it('prepares the admitted 250000 triangles without a value-codec response limit',()=>{
 const positions=new Float32Array(250000*9)
 for(let i=0;i<positions.length;i+=9){positions[i+3]=1;positions[i+7]=1}
 const rendered=renderTriangleSoupInKernel(positions)
 expect(rendered.indices.length).toBe(750000)
 expect(rendered.vertices.length).toBe(4500000)
 expect(rendered.faceIds.at(-1)).toBe(249999)
 expect(rendered.discarded).toBe(0)
 expect(rendered.vertices.slice(-3)).toEqual(Float32Array.from([0,0,1]))
})
