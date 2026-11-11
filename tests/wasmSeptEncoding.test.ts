import {expect,it} from 'vitest'
import {encodeSeptBase118,decodeBase91 as reference,alphabet92} from '../scripts/wasm-base91.mjs'
import {decodeBase91} from '../src/kernel/wasmBase91'
const digits=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92+String.fromCharCode(22,23,24,25,26,28,29,30,31,127)
function word(value:bigint){let out='';for(let i=0;i<7;i++){out+=digits[Number(value%118n)];value/=118n}return out}
it('matches independent BigInt reference across lengths, carries and all byte values',()=>{
 for(const n of [...Array.from({length:258},(_,i)=>i),4096,65537]) {
  for(const bytes of [new Uint8Array(n),new Uint8Array(n).fill(255),Uint8Array.from({length:n},(_,i)=>(i*137+(i>>3))&255)]) {
   const text=encodeSeptBase118(bytes)
   expect(decodeBase91(text)).toEqual(bytes);expect(reference(text)).toEqual(bytes)
   expect(Function(`return '${text}'`)()).toBe(text)
  }
 }
})
it('matches the reference on both sides of48/49-bit width and signed accumulator boundaries',()=>{
 const boundary=118n**7n-2n**48n-1n
 for(const w of [0n,1n,2n**31n-1n,2n**31n,2n**32n-1n,2n**47n,boundary-1n,boundary,boundary+1n,2n**48n-1n,2n**48n,118n**7n-1n]) {
  const text='bKx:0000000c'+word(w)+word(0n)
  expect(decodeBase91(text)).toEqual(reference(text))
 }
})
it('rejects invalid digits, lengths, output overrun and padding even after the last output byte',()=>{
 const packed=encodeSeptBase118(Uint8Array.of(7))
 const bad=['bKx:ffffffff','bKx:00000001'+word(7n+256n),'bKx:00000001'+word(7n+2n**24n),
  'bKx:00000001'+word(7n+2n**48n),packed.slice(0,-1),packed+word(0n),
  'bKx:00000001'+'\x00'.repeat(7),'bKx:00000001'+'é'.repeat(7),
  'bKx:00000007'+word(0n)]
 for(const text of bad){expect(()=>reference(text)).toThrow();expect(()=>decodeBase91(text)).toThrow()}
 expect(()=>decodeBase91(packed,0)).toThrow()
 expect(()=>decodeBase91(encodeSeptBase118(new Uint8Array(100)),99)).toThrow()
})
it('agrees with BigInt acceptance and bytes on deterministic arbitrary frames',()=>{
 let seed=0x3719a23
 const next=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed}
 for(let trial=0;trial<1200;trial++){
  const size=next()%41,groups=next()%7
  const text='bKx:'+size.toString(16).padStart(8,'0')+Array.from({length:groups*7},()=>digits[next()%118]).join('')
  let expected:Uint8Array|undefined
  try{expected=reference(text)}catch{}
  if(expected===undefined)expect(()=>decodeBase91(text)).toThrow()
  else expect(decodeBase91(text)).toEqual(expected)
 }
})
