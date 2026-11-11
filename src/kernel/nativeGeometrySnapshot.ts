import {callGeometryRust} from './geometry/kernel'
import {isNativeGeometryArtifact,type NativeGeometryArtifact} from '../core/nativeGeometry'

/** Storage transport only. Rust restores the exact original JSON bytes; every
 * consumer still audits the decoded actual geometry independently. */
export const packNativeGeometrySnapshot=(json:string):unknown=>callGeometryRust('native_geometry_snapshot_pack',{json})
export function readNativeGeometryData(artifact:NativeGeometryArtifact):any {
 if(!isNativeGeometryArtifact(artifact))throw new Error('Invalid native geometry snapshot')
 const data=JSON.parse(artifact.geometryJson)
 return data?.encoding==='native-deflate-json/1'
  ?JSON.parse(callGeometryRust<string>('native_geometry_snapshot_unpack',{snapshot:data})):data
}
