import type {SolidFace,SolidEdge} from './directSolidTools'
/** Validate mesh-index ownership before topology can drive picking or editing. */
export function validSolidTopology(value:unknown,vertices:number,triangles:number):boolean {
 if(!Number.isSafeInteger(vertices)||vertices<0||!Number.isSafeInteger(triangles)||triangles<0||!value||typeof value!=='object')return false
 const {faces,edges}=value as {faces:SolidFace[];edges:SolidEdge[]}
 if(!Array.isArray(faces)||faces.length>triangles||!Array.isArray(edges)||edges.length>triangles*3)return false
 const index=(n:unknown,limit:number)=>Number.isSafeInteger(n)&&Number(n)>=0&&Number(n)<limit
 const vector=(p:unknown)=>Array.isArray(p)&&p.length===3&&p.every(n=>typeof n==='number'&&Number.isFinite(n))
 const owned=new Set<number>()
 for(const f of faces){
  if(!f||!vector(f.normal)||!vector(f.center)||!Number.isFinite(f.offset)||!Array.isArray(f.triangles)||!f.triangles.length||f.triangles.length>triangles||!Array.isArray(f.vertices)||f.vertices.length<3||f.vertices.length>vertices)return false
  for(const t of f.triangles){if(!index(t,triangles)||owned.has(t))return false;owned.add(t)}
  const unique=new Set<number>()
  for(const v of f.vertices){if(!index(v,vertices)||unique.has(v))return false;unique.add(v)}
 }
 return edges.every(e=>e&&index(e.a,vertices)&&index(e.b,vertices)&&e.a!==e.b&&Array.isArray(e.faces)&&e.faces.length===2&&e.faces[0]!==e.faces[1]&&e.faces.every(f=>index(f,faces.length)))
}
