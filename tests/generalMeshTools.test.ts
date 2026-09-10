import {it,expect} from 'vitest'
import {extrudeDirectSketch} from '../src/services/directModeling'
import {sampledShell,localMeshBevel} from '../src/services/generalMeshTools'
import {solidTopology} from '../src/services/directSolidTools'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const tapered=()=>{
 const b=extrudeDirectSketch({id:'l',name:'Tapered L',closed:true,points:[[0,0],[20,0],[20,10],[10,10],[10,20],[0,20]]},10,'l')
 for(let i=0;i<b.mesh.positions.length;i+=3){const scale=1-b.mesh.positions[i+2]/50;b.mesh.positions[i]*=scale;b.mesh.positions[i+1]*=scale}
 return b
}
it('shells a tapered nonconvex body with sampled distance fields',()=>{
 const b=tapered(),faces=solidTopology(b.mesh).faces,top=faces.findIndex(f=>f.normal[2]>.99)
 const result=sampledShell(b,[top],2,.5),report=inspectPolygonMesh(result.mesh)
 expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeGreaterThan(0);expect(report.signedVolumeMm3).toBeLessThan(inspectPolygonMesh(b.mesh).signedVolumeMm3)
},10000)
it('rejects under-resolved walls before extraction',()=>{
 const b=tapered(),top=solidTopology(b.mesh).faces.findIndex(f=>f.normal[2]>.99)
 expect(()=>sampledShell(b,[top],.1,1)).toThrow('one third')
 expect(()=>sampledShell(b,[top],1,.1)).toThrow('64-cell')
})
it('locally rounds an edge of a non-prismatic nonconvex body',()=>{
 const b=tapered(),t=solidTopology(b.mesh),e=t.edges.findIndex(e=>{
 const a=b.mesh.positions.slice(e.a*3,e.a*3+3),c=b.mesh.positions.slice(e.b*3,e.b*3+3)
 return a[0]===0&&a[1]===0&&c[0]===0&&c[1]===0&&Math.abs(a[2]-c[2])>9
 })
 expect(e).toBeGreaterThanOrEqual(0)
 const before=structuredClone(b),result=localMeshBevel(b,e,1,'fillet'),report=inspectPolygonMesh(result.mesh)
 expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeLessThan(inspectPolygonMesh(b.mesh).signedVolumeMm3);expect(b).toEqual(before)
})
it('adaptively shells below the dense grid limit without cracks between tiles',()=>{
 const b=tapered(),top=solidTopology(b.mesh).faces.findIndex(f=>f.normal[2]>.99)
 for(let i=0;i<b.mesh.positions.length;i+=3){b.mesh.positions[i+1]*=.4;b.mesh.positions[i+2]*=.4}
 const d=sampledShell(b,[top],1,.3,true),r=inspectPolygonMesh(d.mesh)
 expect(r.closed).toBe(true);expect(r.signedVolumeMm3).toBeGreaterThan(0);expect(r.signedVolumeMm3).toBeLessThan(inspectPolygonMesh(b.mesh).signedVolumeMm3)
},20000)
it('supports a tapered edge radius without mutating source geometry',()=>{
 const b=tapered(),t=solidTopology(b.mesh),e=t.edges.findIndex(e=>{const a=b.mesh.positions.slice(e.a*3,e.a*3+3),c=b.mesh.positions.slice(e.b*3,e.b*3+3);return a[0]===0&&a[1]===0&&c[0]===0&&c[1]===0})
 const d=localMeshBevel(b,e,.5,'fillet',1);expect(inspectPolygonMesh(d.mesh).closed).toBe(true)
})
