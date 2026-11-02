import {expect,it,vi} from 'vitest'
vi.mock('../src/generated/geometry-kernels/identity',()=>({default:{sha256:'f'.repeat(64),byteLength:1}}))
it('allows diagnostic module imports while refusing unqualified production routing',async()=>{
 const geometry=await import('../src/core/geometryExecution')
 expect(()=>geometry.ACTIVE_GEOMETRY_MANIFEST_VERSIONS.mesh).toThrow(/No qualified runtime manifest matches/)
 expect(()=>geometry.ACTIVE_GEOMETRY_MANIFEST_VERSIONS.brep).toThrow(/No qualified runtime manifest matches/)
})
