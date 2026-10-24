import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {expect,it} from 'vitest'
import {sameFrame,validPolarProofEvidence} from '../src/services/polarProofProtocol'
const cases=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/variable-annular-self-2026-10-07/final.json.gz',import.meta.url))).toString()).cases
it('accepts original native interval evidence in both directions and radius orders',()=>{
 for(const c of cases)expect(validPolarProofEvidence(c.middlePMatrix)).toBe(true)
})
it('rejects malformed bounds and inflated certificates instead of accepting a positive flag',()=>{
 const proof=cases[0].middlePMatrix
 for(const patch of [{determinantLower:1e9},{determinantLower:NaN},{jacobian:[[[1,0],[0,0]],[[0,0],[1,2]]]},{rowSigns:[0,1]},{diagonalLower:[1e9,1]},{coordinates:[[1,0,0,0],[0,1,0,0],[0,0,NaN,0]]}])expect(validPolarProofEvidence({...proof,...patch})).toBe(false)
 for(const v of [null,undefined,[],{},true])expect(validPolarProofEvidence(v)).toBe(false)
})
it('rejects malformed frame identities without throwing during message validation',()=>{
 const frame=cases[0].middlePMatrix.coordinates
 expect(sameFrame(frame,structuredClone(frame))).toBe(true)
 for(const v of [null,[],{length:3},[null,null,null],['bad','bad','bad']])expect(sameFrame(frame,v)).toBe(false)
})
