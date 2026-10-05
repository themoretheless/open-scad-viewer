import {parseDirectDocument,serializeDirectDocument,type DirectDocument} from './directModeling'
/** Explicit project identity keeps repeated exports independent of filenames. */
export function exportSolidBlenderSnapshot(document:DirectDocument,projectId:string):string {
 if(!projectId.trim()||projectId.length>100)throw Error('A stable project ID is required for Blender updates.')
 if(document.sourceBodies?.length)throw Error('Blender mesh exchange cannot yet display exact source bodies. Keep the native document.')
 const validated=parseDirectDocument(serializeDirectDocument(document))
 if(validated.sketches.length||validated.curves?.length||validated.surfaces?.length)throw Error('Blender mesh exchange currently requires a body-only document.')
 const text=JSON.stringify({schema:'openscad-viewer/blender-1',units:'mm',projectId,bodies:validated.bodies.map(body=>({id:body.id,name:body.name,material:body.material,mesh:{positions:Array.from(body.mesh.positions),indices:Array.from(body.mesh.indices)}}))})
 if(new TextEncoder().encode(text).byteLength>64_000_000)throw Error('Blender snapshot exceeds 64 MB.')
 return text
}
