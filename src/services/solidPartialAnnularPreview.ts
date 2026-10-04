import type {DirectBody} from './directModeling'
import {partialAnnularPreview,tessellateNurbsBrep} from './geometry/brep'

/** Construct display geometry and carry its unresolved qualification explicitly. */
export function solidPartialAnnularPreview(body:DirectBody,edge:number,radius:number) {
 if(!body.brep)throw Error('Select a B-rep body for a circular edge preview.')
 const evidence=partialAnnularPreview(body.brep,edge,radius)
 if(evidence.qualification.status!=='preview-only'||evidence.qualification.commitAllowed!==false)
  throw Error('The partial circular preview returned an unexpected qualification status.')
 const previewBody:DirectBody={...body,brep:evidence.model,mesh:tessellateNurbsBrep(evidence.model,12)}
 return {body:previewBody,evidence}
}
