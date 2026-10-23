import type {StructuralSections,BoundaryConnectivity} from './structuralSections'
const finite = (v:unknown): v is number => typeof v==='number' && Number.isFinite(v)
export function isBoundaryConnectivity(value:unknown,triangles:number,vertices:number):value is BoundaryConnectivity {
  if(!value||typeof value!=='object')return false
  const v=value as BoundaryConnectivity
  if(v.modelKind!=='indexed-edge-boundary-components-v1'||(v.materialConnectivity!=='not-established'&&v.materialConnectivity!=='classified-at-tolerance')
    ||!Array.isArray(v.components)||!v.components.length||v.components.length>triangles
    ||!Array.isArray(v.sharedVertices)||v.sharedVertices.length>vertices)return false
  const seen=new Set<number>()
  for(let i=0;i<v.components.length;i++){
    const c=v.components[i]
    if(!c||c.id!==i||!Number.isFinite(c.signedVolumeMm3)||!Array.isArray(c.sourceTriangles)
      ||!c.sourceTriangles.length||c.sourceTriangles.length>triangles
      ||!Array.isArray(c.boundsMm)||c.boundsMm.length!==2
      ||!c.boundsMm.every(p=>Array.isArray(p)&&p.length===3&&p.every(Number.isFinite))
      ||c.boundsMm[0].some((x,k)=>x>c.boundsMm[1][k]))return false
    for(const t of c.sourceTriangles){
      if(!Number.isInteger(t)||t<0||t>=triangles||seen.has(t))return false
      seen.add(t)
    }
  }
  if(seen.size!==triangles)return false
  const shared=new Set<number>()
  for(const p of v.sharedVertices){
    if(!p||!Number.isInteger(p.sourceVertex)||p.sourceVertex<0||p.sourceVertex>=vertices||shared.has(p.sourceVertex)
      ||!Array.isArray(p.components)||p.components.length<2||p.components.length>v.components.length
      ||!p.components.every((c,i)=>Number.isInteger(c)&&c>=0&&c<v.components.length&&(i===0||c>p.components[i-1])))return false
    shared.add(p.sourceVertex)
  }
  return true
}

export function isMaterialAudit(v:StructuralSections):boolean {
  const a=v.materialAudit
  if(!a||typeof a!=='object')return false
  if(a.status==='unresolved')return v.selfIntersections==='not-checked'
    &&v.connectivity.materialConnectivity==='not-established'
    &&typeof a.code==='string'&&a.code.length>0&&a.code.length<=128
    &&typeof a.message==='string'&&a.message.length>0&&a.message.length<=4096
  if(a.status!=='classified'||v.selfIntersections!=='checked-at-tolerance'
    ||v.connectivity.materialConnectivity!=='classified-at-tolerance'||v.connectivity.sharedVertices.length!==0
    ||!finite(a.toleranceMm)||a.toleranceMm<=0||!Number.isInteger(a.materialRegions)||a.materialRegions<=0
    ||!Array.isArray(a.shells)||a.shells.length!==v.connectivity.components.length||a.shells.length>128)return false
  if(!a.shells.every((s,i)=>s&&s.id===i&&s.firstTriangle===v.connectivity.components[i].sourceTriangles[0]
    &&Number.isInteger(s.depth)&&s.depth>=0&&s.depth<a.shells.length
    &&s.kind===(s.depth%2===0?'material':'cavity')
    &&(s.parent===null?s.depth===0:Number.isInteger(s.parent)&&s.parent>=0&&s.parent<a.shells.length&&s.parent!==i)))return false
  return a.shells.every(s=>s.parent===null||a.shells[s.parent].depth+1===s.depth)
    &&a.materialRegions===a.shells.filter(s=>s.kind==='material').length
}
