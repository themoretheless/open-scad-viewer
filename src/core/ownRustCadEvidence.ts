// Recorded exact artifacts; no cross-host byte identity or clean qualification is claimed.
import packagedArtifact from '../generated/geometry-kernels/identity'
export const OWN_RUST_CAD_ARTIFACTS = Object.freeze([
  {
    "sha256": "a3a170bc43df9fcd307be2147d52ed2652cfa66ab95122b1bac5a2215a5d4596",
    "byteLength": 12833089
  }
])
export function recordedOwnRustCadFingerprint(artifact: {sha256:string;byteLength:number}): string|null {
  if (!Number.isSafeInteger(artifact.byteLength) || artifact.byteLength <= 0 || artifact.byteLength > 16 * 1024 * 1024) return null
  return OWN_RUST_CAD_ARTIFACTS.find(item => item.sha256 === artifact.sha256 && (item.byteLength === null || item.byteLength === artifact.byteLength))?.sha256 ?? null
}
export const OWN_RUST_CAD_EVIDENCE = Object.freeze({
  // Runtime identity follows packaged bytes; qualification freezing separately rejects unrecorded artifacts.
  "wasmSha256": packagedArtifact.sha256,
  "noticesSha256": "789499bf4bcbaacad6ead9d5e8f51e5ecbd3ec37bb9b9ef2d45bb7c115bd5c84",
  "lockfileSha256": "8e84112c568fc204b2db0906d7e20860a3e28af3a3df92c18af994f485b87271",
  "rustLockfileSha256": "db822b6ec328b4b32ec4ca1b386ddef0ecc94c190ada4734873f2cbccd9248b7"
})
