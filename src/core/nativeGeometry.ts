import {sha256Hex} from './sha256'
export const MAX_NATIVE_GEOMETRY_CHARACTERS = 4 * 1024 * 1024
/** Immutable serialized f64 geometry, separate from the Float32 rendering payload.
 * Geometry is revalidated by its owning kernel before further operations.
 * Revision identity is snapshot-local, not a persistent topology naming claim.
 */
export interface NativeGeometryArtifact {
  readonly version:1
  readonly nodeId:string
  readonly kind:'profile'|'patches'|'subdivision'|'sdf'|'brep'|'curve'|'surface'|'mesh'
  readonly revision:string
  readonly geometryJson:string
  readonly documentJson:string
  readonly documentRevision:string
}
const kinds=['profile','patches','subdivision','sdf','brep','curve','surface','mesh']
export function createNativeGeometryArtifact(nodeId:string,kind:NativeGeometryArtifact['kind'],data:unknown,document:unknown):NativeGeometryArtifact {
  const geometryJson=JSON.stringify(data),documentJson=JSON.stringify(document)
  const artifact={version:1 as const,nodeId,kind,revision:sha256Hex(kind+'\n'+geometryJson),geometryJson,documentJson,documentRevision:sha256Hex(documentJson)}
  if(!isNativeGeometryArtifact(artifact))throw new Error('Native geometry snapshot exceeds its bounded transport contract')
  return Object.freeze(artifact)
}
export function isNativeGeometryArtifact(value:unknown):value is NativeGeometryArtifact {
  if(!value||typeof value!=='object'||Array.isArray(value))return false
  const keys=['version','nodeId','kind','revision','geometryJson','documentJson','documentRevision']
  const descriptors=Object.getOwnPropertyDescriptors(value)
  if(Reflect.ownKeys(value).length!==keys.length||!keys.every(k=>descriptors[k]&&'value' in descriptors[k]))return false
  const v=value as NativeGeometryArtifact
  return v.version===1&&typeof v.nodeId==='string'&&v.nodeId.length>0&&v.nodeId.length<=128
    &&kinds.includes(v.kind)&&typeof v.geometryJson==='string'&&typeof v.documentJson==='string'
    &&v.geometryJson.length+v.documentJson.length<=MAX_NATIVE_GEOMETRY_CHARACTERS
    &&v.revision===sha256Hex(v.kind+'\n'+v.geometryJson)&&v.documentRevision===sha256Hex(v.documentJson)
}
export interface NativeFaceReference {nodeId:string;revision:string;kind:NativeGeometryArtifact['kind'];faceId:number}
/** Reject stale selection explicitly; numeric face IDs are meaningful only within a revision. */
export function nativeFaceReference(artifact:NativeGeometryArtifact,faceId:number):NativeFaceReference {
  if(!Number.isSafeInteger(faceId)||faceId<0)throw new Error('Invalid native face ID')
  return Object.freeze({nodeId:artifact.nodeId,revision:artifact.revision,kind:artifact.kind,faceId})
}
export function assertNativeReferenceCurrent(artifact:NativeGeometryArtifact,reference:NativeFaceReference):void {
  if(reference.nodeId!==artifact.nodeId||reference.kind!==artifact.kind||reference.revision!==artifact.revision)throw new Error('Native geometry selection is stale')
}
