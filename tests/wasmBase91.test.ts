import {expect,it} from 'vitest'
import {encodeBase91,encodeVariableBase91,encodeVariableBase92,encodeVariableBase93,encodeVariableBase94,encodeVariableBase95,encodeVariableBase108,encodeVariableBase112,encodeVariableBase113,encodeTripleBase113,encodeTripleBase114,encodeTripleBase117,encodeTripleBase118,alphabet,alphabet92,decodeBase91 as decodeBuildBase91} from '../scripts/wasm-base91.mjs'
import {decodeBase91} from '../src/services/wasmBase91'
it.each([encodeVariableBase91,encodeVariableBase92,encodeVariableBase93,encodeVariableBase94,encodeVariableBase95,encodeVariableBase108,encodeVariableBase112,encodeVariableBase113])('variable pairs preserve all word values, alignments, zeros and quoted literals (%#)',encode=>{
 const corpus=Uint8Array.from({length:65536*2},(_,i)=>i%2?Math.floor(i/2)>>8:Math.floor(i/2)&255)
 for(const bytes of [corpus,...Array.from({length:258},(_,n)=>new Uint8Array(n)),...Array.from({length:258},(_,n)=>Uint8Array.from({length:n},(_,i)=>(i*137+(i>>3))&255))]){
  const packed=encode(bytes)
  expect(decodeBase91(packed)).toEqual(bytes)
  expect(decodeBuildBase91(packed)).toEqual(bytes)
  expect(Function(`return '${packed}'`)()).toBe(packed)
  expect(()=>decodeBase91(packed+'!!')).toThrow()
  if(bytes.length){expect(()=>decodeBase91(packed.slice(0,-2))).toThrow();expect(()=>decodeBase91(packed,bytes.length-1)).toThrow()}
 }
 expect(encodeVariableBase91(corpus).length).toBeLessThan(encodeBase91(corpus).length)
 for(const packed of ['b9v:ffffffff','b9v:00000001é!','b9v:00000001!!a','b9v:00000001'+alphabet[90]+alphabet[90]])expect(()=>decodeBase91(packed)).toThrow()
})
it('round trips all bit alignments and arbitrary bytes in a quoted JS literal',()=>{
 for(const n of [...Array.from({length:257},(_,i)=>i),4096,10001]){
  const bytes=Uint8Array.from({length:n},(_,i)=>(i*137+(i>>3))&255),encoded=encodeBase91(bytes)
  expect(decodeBase91(encoded)).toEqual(bytes)
  expect(decodeBuildBase91(encoded)).toEqual(bytes)
  expect(Function(`return '${encoded}'`)()).toBe(encoded)
  expect(encoded.length).toBe(12+2*Math.ceil(n*8/13))
 }
 expect(alphabet.length).toBe(91)
})
it('rejects truncation, oversized allocation, invalid digits and padding aliases',()=>{
 const s=encodeBase91(Uint8Array.of(7))
 for(const bad of [s.slice(0,-1),s+'!!','b91:ffffffff','b91:00000001é!',s.slice(0,12)+alphabet[90]+alphabet[90]])expect(()=>decodeBase91(bad)).toThrow()
 expect(()=>decodeBase91(s,0)).toThrow()
 const word=7+256,padding=s.slice(0,12)+alphabet[word%91]+alphabet[Math.floor(word/91)]
 expect(()=>decodeBase91(padding)).toThrow('padding')
})
it('rejects malformed base92 lengths, unsafe digits and nonzero padding',()=>{
 expect(alphabet92).toHaveLength(92)
 expect(alphabet92).toContain('`')
 const packed=encodeVariableBase92(Uint8Array.of(7)),word=7+256
 const padding=packed.slice(0,12)+alphabet92[word%92]+alphabet92[Math.floor(word/92)]
 for(const value of ['b92:ffffffff','b92:00000001é!','b92:00000001\\!','b92:00000001\'!',padding,packed+'!!',packed.slice(0,-2)]){
  expect(()=>decodeBase91(value)).toThrow()
  expect(()=>decodeBuildBase91(value)).toThrow()
 }
})

it('preserves base93 spaces and rejects unsafe characters and padding',()=>{
 const alphabet93=' '+alphabet92,packed=encodeVariableBase93(Uint8Array.of(7)),word=263
 expect(encodeVariableBase93(new Uint8Array(4))).toContain(' ')
 const padding=packed.slice(0,12)+alphabet93[word%93]+alphabet93[Math.floor(word/93)]
 for(const value of ['b93:ffffffff','b93:00000001é!',padding,packed+'  ',packed.slice(0,-2)]){
  expect(()=>decodeBase91(value)).toThrow()
  expect(()=>decodeBuildBase91(value)).toThrow()
 }
})

it('preserves base94 tabs and refuses malformed lengths, unsafe digits and padding',()=>{
 const digits='\t '+alphabet92,packed=encodeVariableBase94(Uint8Array.of(7)),word=263
 expect(encodeVariableBase94(new Uint8Array(4))).toContain('\t')
 const padding=packed.slice(0,12)+digits[word%94]+digits[Math.floor(word/94)]
 for(const bad of ['b94:ffffffff','b94:00000001é!','b94:00000001\n!',padding,packed+'\t\t',packed.slice(0,-2)]){
  expect(()=>decodeBase91(bad)).toThrow()
  expect(()=>decodeBuildBase91(bad)).toThrow()
 }
})

it('preserves base95 SOH and refuses malformed digits and padding',()=>{
 const digits='\x01\t '+alphabet92,packed=encodeVariableBase95(Uint8Array.of(7)),word=263
 expect(encodeVariableBase95(new Uint8Array(4))).toContain('\x01')
 const padding=packed.slice(0,12)+digits[word%95]+digits[Math.floor(word/95)]
 for(const bad of ['b95:ffffffff','b95:00000001é!','b95:00000001\n!',padding,packed+'\x01\x01',packed.slice(0,-2)]){
  expect(()=>decodeBase91(bad)).toThrow()
  expect(()=>decodeBuildBase91(bad)).toThrow()
 }
})

it('refuses base108 control aliases, overflow, truncation and nonzero padding',()=>{
 const digits=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92
 const packed=encodeVariableBase108(Uint8Array.of(7)),word=263
 const padding=packed.slice(0,12)+digits[word%108]+digits[Math.floor(word/108)]
 for(const bad of ['bAx:ffffffff','bAx:00000001\n!','bAx:00000001\x07!',packed.slice(0,-2),packed+'!!',padding]){
  expect(()=>decodeBase91(bad)).toThrow()
  expect(()=>decodeBuildBase91(bad)).toThrow()
 }
})

it('rejects malformed base112 controls, sizes and padding after JS emission',async()=>{
 const {transformSync}=await import('esbuild')
 const bytes=Uint8Array.from({length:131072},(_,i)=>(i*137+(i>>3))&255)
 const packed=encodeVariableBase112(bytes)
 const emitted=transformSync(`globalThis.__packed='${packed}'`,{minify:true,target:'es2022'}).code
 const restored=Function(`${emitted};return globalThis.__packed`)()
 delete (globalThis as Record<string,unknown>).__packed
 expect(restored).toBe(packed)
 expect(decodeBase91(restored)).toEqual(bytes)
 expect(decodeBuildBase91(restored)).toEqual(bytes)
 const digits=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92+String.fromCharCode(22,23,24,25)
 const one=encodeVariableBase112(Uint8Array.of(7)),word=263
 const padding=one.slice(0,12)+digits[word%112]+digits[Math.floor(word/112)]
 for(const bad of ['bBx:ffffffff','bBx:00000001\n!','bBx:00000001\x1a!',one.slice(0,-2),one+'!!',padding]){
  expect(()=>decodeBase91(bad)).toThrow()
  expect(()=>decodeBuildBase91(bad)).toThrow()
 }
})

it('preserves base113 emission and refuses control aliases and padding',async()=>{
 const {transformSync}=await import('esbuild')
 const bytes=Uint8Array.from({length:131072},(_,i)=>(i*137+(i>>3))&255)
 const packed=encodeVariableBase113(bytes)
 const emitted=transformSync(`returnValue='${packed}'`,{minify:true,target:'es2022'}).code
 const restored=Function(`let returnValue;${emitted};return returnValue`)()
 expect(restored).toBe(packed)
 expect(decodeBase91(restored)).toEqual(bytes)
 expect(decodeBuildBase91(restored)).toEqual(bytes)
 const one=encodeVariableBase113(Uint8Array.of(7))
 const digits=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92+String.fromCharCode(22,23,24,25,26),word=263
 for(const bad of ['bCx:ffffffff','bCx:00000001\n!','bCx:00000001\x1b!',one.slice(0,-2),one+'!!',one.slice(0,12)+digits[word%113]+digits[Math.floor(word/113)]]){
  expect(()=>decodeBase91(bad)).toThrow()
  expect(()=>decodeBuildBase91(bad)).toThrow()
 }
})

it.each([encodeTripleBase113,encodeTripleBase114,encodeTripleBase117,encodeTripleBase118])('roundtrips triple-word boundaries, JS emission and refuses incomplete or aliased words (%#)',async encode=>{
 const {transformSync}=await import('esbuild')
 const base=encode===encodeTripleBase113?113:encode===encodeTripleBase114?114:encode===encodeTripleBase117?117:118
 const digits=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92+String.fromCharCode(22,23,24,25,26)+(base===114?String.fromCharCode(28):base===117?String.fromCharCode(28,29,30,31):base===118?String.fromCharCode(28,29,30,31,127):'')
 const corpus=Uint8Array.from({length:3*1048576},(_,i)=>(Math.floor(i/3)>>(8*(i%3)))&255)
 for(const bytes of [corpus,...Array.from({length:259},(_,n)=>new Uint8Array(n)),...Array.from({length:259},(_,n)=>Uint8Array.from({length:n},(_,i)=>(i*137+(i>>3))&255))]){
  const packed=encode(bytes)
  expect(decodeBase91(packed)).toEqual(bytes)
  expect(decodeBuildBase91(packed)).toEqual(bytes)
  if(bytes.length)expect(()=>decodeBase91(packed,bytes.length-1)).toThrow()
 }
 const packed=encode(corpus.subarray(0,65536))
 const emitted=transformSync(`returnValue='${packed}'`,{minify:true,target:'es2022'}).code
 expect(Function(`let returnValue;${emitted};return returnValue`)()).toBe(packed)
 const one=encode(Uint8Array.of(7)),word=263
 const padding=one.slice(0,12)+digits[word%base]+digits[Math.floor(word/base)%base]+digits[Math.floor(word/(base*base))]
 for(const bad of [one.slice(0,4)+'ffffffff',one.slice(0,4)+'00000001\n!!',one.slice(0,4)+'00000001\x1b!!',one.slice(0,-1),one.slice(0,-3),one+'!!!',padding]){
  expect(()=>decodeBase91(bad)).toThrow()
  expect(()=>decodeBuildBase91(bad)).toThrow()
 }
})
