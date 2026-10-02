import {describe,expect,it} from 'vitest'
import {OWN_RUST_CAD_ARTIFACTS,recordedOwnRustCadFingerprint} from '../src/core/ownRustCadEvidence'

describe('recorded own Rust CAD artifacts',()=>{
 it('admits only explicitly recorded hashes and their measured lengths',()=>{
  for(const artifact of OWN_RUST_CAD_ARTIFACTS){
   const byteLength=artifact.byteLength??9676259
   expect(recordedOwnRustCadFingerprint({sha256:artifact.sha256,byteLength})).toBe(artifact.sha256)
   if(artifact.byteLength!==null)expect(recordedOwnRustCadFingerprint({sha256:artifact.sha256,byteLength:byteLength+1})).toBeNull()
  }
 })
 it('refuses an unknown hash and unbounded or invalid artifact sizes',()=>{
  expect(recordedOwnRustCadFingerprint({sha256:'0'.repeat(64),byteLength:9676259})).toBeNull()
  for(const byteLength of [0,-1,1.5,NaN,16*1024*1024+1]){
   expect(recordedOwnRustCadFingerprint({sha256:OWN_RUST_CAD_ARTIFACTS[0].sha256,byteLength})).toBeNull()
  }
 })
})
