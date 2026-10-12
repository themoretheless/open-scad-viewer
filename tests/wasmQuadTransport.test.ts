import {expect,it} from 'vitest'
import {encodeQuadBase118,alphabet92,decodeBase91 as buildDecode} from '../scripts/wasm-base91.mjs'
import {decodeBase91} from '../src/services/wasmBase91'
const digits=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92+String.fromCharCode(22,23,24,25,26,28,29,30,31,127)
// Independent arbitrary-integer bit stream, including word cuts above bit31.
function oracle(bytes:Uint8Array){
 let stream=0n;for(let i=bytes.length-1;i>=0;i--)stream=(stream<<8n)|BigInt(bytes[i])
 let remaining=bytes.length*8,text='bJx:'+bytes.length.toString(16).padStart(8,'0')
 const mask=(1n<<27n)-1n,threshold=BigInt(118**4)-mask-2n
 while(remaining>0){let width=remaining>27&&((stream&mask)<=threshold)?28:27;let word=Number(stream&((1n<<BigInt(width))-1n));for(let j=0;j<4;j++){text+=digits[word%118];word=Math.floor(word/118)}stream>>=BigInt(width);remaining-=width}
 return text
}
it('preserves independent word thresholds, byte alignments and all final lengths',()=>{
 const corpus:Uint8Array[]=[]
 for(let n=0;n<=258;n++)for(const mode of [0,1,2])corpus.push(Uint8Array.from({length:n},(_,i)=>mode===0?0:mode===1?255:(i*137+n*29+(i>>3))&255))
 const threshold=118**4-2**27-1
 for(let shift=0;shift<8;shift++)for(const word of [0,1,threshold-1,threshold,threshold+1,2**27-1,2**27,118**4-1]){
  let value=(BigInt(word)<<BigInt(shift))|((1n<<BigInt(shift))-1n);const bytes=new Uint8Array(16);for(let i=0;i<16;i++){bytes[i]=Number(value&255n);value>>=8n}corpus.push(bytes)
 }
 for(const bytes of corpus){const encoded=encodeQuadBase118(bytes);expect(encoded).toBe(oracle(bytes));expect(decodeBase91(encoded)).toEqual(bytes);expect(buildDecode(encoded)).toEqual(bytes);expect(Function(`return '${encoded}'`)()).toBe(encoded)}
})
it('refuses invalid digits, allocation limits, incomplete words and padding aliases',()=>{
 const text=encodeQuadBase118(Uint8Array.of(7)),padding='bJx:00000001'+digits[263%118]+digits[Math.floor(263/118)]+digits[0]+digits[0]
 for(const decode of [decodeBase91,buildDecode]){
  expect(()=>decode(text,0)).toThrow()
  for(const bad of [text.slice(0,-1),text+digits[0].repeat(4),'bJx:ffffffff','bJx:00000001é!!!','bJx:00000001\n!!!',padding])expect(()=>decode(bad)).toThrow()
 }
})
