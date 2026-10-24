// Recorded exact artifacts; no cross-host byte identity or clean qualification is claimed.
import packagedArtifact from '../generated/geometry-kernels/identity'
export const OWN_RUST_CAD_ARTIFACTS = Object.freeze([
{
  "sha256": "2e0e21416b1266384f3b33c18b5de381220c89ee10449b7c4f847361eda120a9",
  "byteLength": 12834623
},
  {
    "sha256": "14ab9ca344465d272de4d402118b88a138ff574dd29dad9cbabe86ffbc4368a4",
    "byteLength": 12142979
  },
  {
    "sha256": "b009f02f131014a271a2140a892590c409f23d865c66eb21ffa94352edd8a844",
    "byteLength": 13749770
  },
  {
    "sha256": "2ff06f662379e92193ff80849ce03bac9ab28c82d00223ff9a48e4d0307ca058",
    "byteLength": 13751449
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
  "rustLockfileSha256": "0c68303e41d40ad569ee29e786b134fc6d51753f51bc3e09d3a71421b63bf369"
})
