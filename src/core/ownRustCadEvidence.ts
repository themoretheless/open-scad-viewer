// Recorded exact artifacts; no cross-host byte identity or clean qualification is claimed.
import packagedArtifact from '../generated/geometry-kernels/identity'
export const OWN_RUST_CAD_ARTIFACTS = Object.freeze([
  {
    "sha256": "54539bb039b8047ba4bc5c030d7361655c0172f58ec85c78c95da106de146cf6",
    "byteLength": 9676259
  },
  {
    "sha256": "58889ee35bbf97aa6bb7f8bd01404928668cdddccec7a349934b797fe8d88d5c",
    "byteLength": null
  },
  {
    "sha256": "c30474a3b4b6ddf0134d59a897d3b1dd1d2e93dc9c15adebdf5db47b94768b5c",
    "byteLength": 9676099
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
  "rustLockfileSha256": "ba03ef15f1a195c613fa74f9393843d49826d0b195305b540cd63c77db847c2c"
})
