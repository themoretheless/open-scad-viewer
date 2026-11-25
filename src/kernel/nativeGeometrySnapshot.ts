import { callGeometryRust } from './geometry/kernel'
import { isNativeGeometryArtifact, type NativeGeometryArtifact } from '../core/nativeGeometry'

/** Storage transport only. Rust restores the exact original JSON bytes; every
 * consumer still audits the decoded actual geometry independently. */
export const packNativeGeometrySnapshot = (json: string): unknown =>
  callGeometryRust('native_geometry_snapshot_pack', { json })
/** Decoded JSON from Rust or storage before validation. Validators probe it field by field
 * and reject on any mismatch, so the shape is deliberately open until checked. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any -- probed field by field before use
export type ProbedJson = any
export function readNativeGeometryData(artifact: NativeGeometryArtifact): ProbedJson {
  if (!isNativeGeometryArtifact(artifact)) throw new Error('Invalid native geometry snapshot')
  const data = JSON.parse(artifact.geometryJson)
  return (data?.encoding === 'native-deflate-json/1' || data?.encoding === 'native-brotli-json/1')
    ? JSON.parse(callGeometryRust<string>('native_geometry_snapshot_unpack', { snapshot: data }))
    : data
}
