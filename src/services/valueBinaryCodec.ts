/** MGV1: bounded, versioned, little-endian values. No JSON text at the WASM boundary. */
const LIMIT=32*1024*1024, encoder=new TextEncoder(),decoder=new TextDecoder('utf-8',{fatal:true})
/** Per-key hints: arrays of numeric triples decode directly into flat typed arrays. */
export type BinaryTripleHints=Readonly<Record<string,'f64'|'u8'|'u32'>>
export function encodeBinary(value:unknown):Uint8Array {
 let data=new Uint8Array(1024),view=new DataView(data.buffer),offset=4,nodes=0
 data.set([77,71,86,49])
 const reserve=(n:number)=>{if(n>LIMIT-offset)throw new Error('Binary size limit');if(offset+n>data.length){const next=new Uint8Array(Math.min(LIMIT,Math.max(offset+n,data.length*2)));next.set(data);data=next;view=new DataView(data.buffer)}}
 const byte=(n:number)=>{reserve(1);data[offset++]=n}
 const count=(n:number)=>{reserve(4);view.setUint32(offset,n,true);offset+=4}
 const put=(v:unknown,depth:number)=>{
  if(depth>128||++nodes>4_000_000)throw new Error('Binary nesting or item limit')
  if(v===null){byte(0);return}if(v===false){byte(1);return}if(v===true){byte(2);return}
  if(typeof v==='number'){if(!Number.isFinite(v))throw new Error('Nonfinite binary number');const integer=Number.isSafeInteger(v)&&!Object.is(v,-0);byte(integer?(v>=0?7:8):3);reserve(8);if(integer){if(v>=0)view.setBigUint64(offset,BigInt(v),true);else view.setBigInt64(offset,BigInt(v),true)}else view.setFloat64(offset,v,true);offset+=8;return}
  if(typeof v==='string'){const bytes=encoder.encode(v);byte(4);count(bytes.length);reserve(bytes.length);data.set(bytes,offset);offset+=bytes.length;return}
  if(Array.isArray(v)){byte(5);count(v.length);for(const item of v)put(item,depth+1);return}
  if(typeof v==='object'&&v){byte(6);const entries=Object.entries(v).filter(([,v])=>v!==undefined);count(entries.length);for(const[k,item]of entries){put(k,depth+1);put(item,depth+1)}return}
  throw new Error('Unsupported binary value')
 }
 put(value,0);return data.slice(0,offset)
}
export function decodeBinary(data:Uint8Array,hints?:BinaryTripleHints):unknown {
 if(data.length>LIMIT||data.length<4||data[0]!==77||data[1]!==71||data[2]!==86||data[3]!==49)throw new Error('Invalid binary header or size')
 const view=new DataView(data.buffer,data.byteOffset,data.byteLength);let offset=4,nodes=0
 const need=(n:number)=>{if(n>data.length-offset)throw new Error('Truncated binary value')}
 const count=()=>{need(4);const n=view.getUint32(offset,true);offset+=4;return n}
 // Fast path for the fixed 32-byte rows written by the photo adapter; any deviation rewinds to the generic walk.
 const triples=(n:number,kind:'f64'|'u8'|'u32')=>{
  if(n>(4_000_000-nodes)/4)throw new Error('Binary item limit')
  const start=offset
  if(n*32>data.length-offset)return undefined
  const out:Float64Array|Uint8Array|Uint32Array=kind==='f64'?new Float64Array(n*3):kind==='u8'?new Uint8Array(n*3):new Uint32Array(n*3)
  for(let i=0;i<n;i++){
   if(data[offset]!==5||view.getUint32(offset+1,true)!==3){offset=start;return undefined}
   offset+=5
   for(let j=0;j<3;j++){
    const tag=data[offset++]
    if(kind==='f64'){if(tag===3){out[i*3+j]=view.getFloat64(offset,true);offset+=8}else if(tag===0)out[i*3+j]=NaN;else{offset=start;return undefined}}
    else{if(tag!==7){offset=start;return undefined}const low=view.getUint32(offset,true),high=view.getUint32(offset+4,true);offset+=8;if(high!==0||(kind==='u8'&&low>255)){offset=start;return undefined}out[i*3+j]=low}
   }
  }
  nodes+=n*4;return out
 }
 const get=(depth:number,hint?:'f64'|'u8'|'u32'):unknown=>{
  if(depth>128||++nodes>4_000_000)throw new Error('Binary nesting or item limit');need(1);const tag=data[offset++]
  if(tag===0)return null;if(tag===1)return false;if(tag===2)return true
  if(tag===3||tag===7||tag===8){need(8);const n=tag===3?view.getFloat64(offset,true):Number(tag===7?view.getBigUint64(offset,true):view.getBigInt64(offset,true));offset+=8;if(!Number.isFinite(n)||(tag!==3&&!Number.isSafeInteger(n)))throw new Error('Invalid binary number');return n}
  if(tag===4){const n=count();need(n);const s=decoder.decode(data.subarray(offset,offset+n));offset+=n;return s}
  if(tag===5||tag===6){const n=count();if(n>data.length-offset||n>4_000_000-nodes)throw new Error('Binary item limit');if(tag===5){if(hint){const flat=triples(n,hint);if(flat)return flat}const out:unknown[]=[];for(let i=0;i<n;i++)out.push(get(depth+1));return out}const out:Record<string,unknown>={};for(let i=0;i<n;i++){const k=get(depth+1);if(typeof k!=='string'||Object.hasOwn(out,k))throw new Error('Invalid or duplicate binary object key');Object.defineProperty(out,k,{value:get(depth+1,hints?.[k]),enumerable:true,writable:true,configurable:true})}return out}
  throw new Error('Invalid binary tag')
 }
 const value=get(0);if(offset!==data.length)throw new Error('Trailing binary data');return value
}
