/** Bounded raw DEFLATE decoder for generated WASM. No runtime package or network load. */
const codeOrder=[16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15]
const lengthBase=[3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258]
const lengthExtra=[0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0]
const distanceBase=[1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577]
const distanceExtra=[0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13]
type Tree={table:Uint32Array;bits:number}
function tree(lengths:number[]):Tree{
 const counts=new Uint32Array(16);let max=0
 for(const n of lengths){if(n<0||n>15)throw new Error('Invalid Huffman length');if(n){counts[n]=counts[n]!+1;max=Math.max(max,n)}}
 if(!max)return {table:new Uint32Array(1),bits:0}
 let slots=1;for(let n=1;n<=max;n++){slots=slots*2-counts[n]!;if(slots<0)throw new Error('Oversubscribed Huffman tree')}
 const next=new Uint32Array(16);let code=0;for(let n=1;n<=max;n++){code=(code+counts[n-1]!)*2;next[n]=code}
 const table=new Uint32Array(1<<max)
 lengths.forEach((n,symbol)=>{if(!n)return;let v=next[n]!,reversed=0;next[n]=v+1;for(let j=0;j<n;j++){reversed=reversed*2+(v&1);v>>>=1}for(let j=reversed;j<table.length;j+=1<<n)table[j]=(symbol<<5)|n})
 return {table,bits:max}
}
const fixedLiteral=tree(Array.from({length:288},(_,n)=>n<144?8:n<256?9:n<280?7:8)),fixedDistance=tree(Array(32).fill(5))
export function unpackWasm(input:Uint8Array):Uint8Array<ArrayBuffer>{
 if(input.length<4)throw new Error('Truncated WASM package')
 const size=new DataView(input.buffer,input.byteOffset,input.byteLength).getUint32(0,true)
 if(size>16*1024*1024)throw new Error('WASM package limit')
 const output=new Uint8Array(size);let i=4,o=0,bits=0,available=0
 const fill=(n:number)=>{while(available<n&&i<input.length){bits|=input[i++]!<<available;available+=8}}
 const read=(n:number)=>{fill(n);if(available<n)throw new Error('Truncated DEFLATE bits');const value=bits&((1<<n)-1);bits>>>=n;available-=n;return value}
 const symbol=(t:Tree)=>{fill(t.bits);const entry=t.table[bits&((1<<t.bits)-1)]!,n=entry&31;if(!n||n>available)throw new Error('Invalid DEFLATE code');bits>>>=n;available-=n;return entry>>>5}
 let final=0
 do{
  final=read(1);const type=read(2)
  if(type===0){read(available%8);const n=read(16),inverse=read(16);if((n^inverse)!==65535||n>size-o)throw new Error('Invalid stored block');for(let j=0;j<n;j++)output[o++]=read(8);continue}
  if(type===3)throw new Error('Invalid DEFLATE block type')
  let lit=fixedLiteral,dist=fixedDistance
  if(type===2){const nl=read(5)+257,nd=read(5)+1,nc=read(4)+4;const lengths=Array(19).fill(0);for(let j=0;j<nc;j++)lengths[codeOrder[j]!]=read(3);const ct=tree(lengths),all:number[]=[]
   while(all.length<nl+nd){const s=symbol(ct);if(s<16){all.push(s);continue}let n:number,v=0;if(s===16){if(!all.length)throw new Error('Missing Huffman repeat');v=all[all.length-1]!;n=read(2)+3}else if(s===17)n=read(3)+3;else if(s===18)n=read(7)+11;else throw new Error('Invalid Huffman repeat');if(n>nl+nd-all.length)throw new Error('Huffman repeat overflow');for(let j=0;j<n;j++)all.push(v)}
   if(!all[256])throw new Error('Missing end-of-block code');lit=tree(all.slice(0,nl));dist=tree(all.slice(nl))
  }
  for(;;){const s=symbol(lit);if(s===256)break;if(s<256){if(o===size)throw new Error('WASM output overflow');output[o++]=s;continue}
   if(s>285)throw new Error('Invalid match length');const length=lengthBase[s-257]!+read(lengthExtra[s-257]!);const d=symbol(dist);if(d>29)throw new Error('Invalid match distance');const distance=distanceBase[d]!+read(distanceExtra[d]!);if(distance>o||length>size-o)throw new Error('WASM match out of bounds');for(let j=0;j<length;j++){output[o]=output[o-distance]!;o++}
  }
 }while(!final)
 if(o!==size||i-Math.floor(available/8)!==input.length)throw new Error('WASM package size mismatch')
 return output
}
