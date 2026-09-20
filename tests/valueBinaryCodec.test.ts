import {describe,it,expect,vi} from 'vitest'
import {deflateRawSync,constants} from 'node:zlib'
import {encodeBinary,decodeBinary} from '../src/services/valueBinaryCodec'
import {unpackWasm} from '../src/services/wasmPacking'
import {callGeometryRust,createRustSurfaceEvaluator} from '../src/services/geometry/kernel'
import {prepareGraphRust} from '../src/services/languages/kernel'

describe('dependency-free WASM boundary',()=>{
 it('preserves safe signed/unsigned 64-bit boundaries and refuses unsafe integers',()=>{
  const bytes=new Uint8Array(13),view=new DataView(bytes.buffer)
  bytes.set([77,71,86,49])
  const values=[0n,1n,-1n,2147483647n,2147483648n,4294967295n,4294967296n,-4294967296n,-4294967297n,9007199254740991n,-9007199254740991n,9007199254740992n,-9007199254740992n,9223372036854775807n,-9223372036854775808n,18446744073709551615n]
  let seed=19
  for(let i=0;i<1000;i++){
   seed=(Math.imul(seed,1664525)+1013904223)>>>0
   const high=seed
   seed=(Math.imul(seed,1664525)+1013904223)>>>0
   values.push((BigInt(high)<<32n)|BigInt(seed))
   const safe=(BigInt(high&0x1fffff)<<32n)|BigInt(seed)
   values.push(safe,-safe)
  }
  for(const value of values)for(const tag of [7,8]){
   bytes[4]=tag
   view.setBigUint64(5,BigInt.asUintN(64,value),true)
   const expected=Number(tag===7?view.getBigUint64(5,true):view.getBigInt64(5,true))
   if(Number.isSafeInteger(expected))expect(decodeBinary(bytes)).toBe(expected)
   else expect(()=>decodeBinary(bytes)).toThrow('Invalid binary number')
  }
 })
 it('round-trips Unicode, numbers, arrays and own prototype-like keys',()=>{
  const value={text:'Русский 😀\u0000',values:[null,true,false,-0,0,-1,1.5,Number.MAX_SAFE_INTEGER],['__proto__']:{safe:true}}
  const restored=decodeBinary(encodeBinary(value))
  expect(restored).toEqual(value);expect(Object.getPrototypeOf(restored)).toBe(Object.prototype)
 })
 it('rejects every truncation, invalid types, trailing data and deep recursion',()=>{
  const bytes=encodeBinary({a:[1,2,'xyz']})
  for(let i=0;i<bytes.length;i++)expect(()=>decodeBinary(bytes.subarray(0,i))).toThrow()
  expect(()=>decodeBinary(new Uint8Array([...bytes,0]))).toThrow('Trailing')
  expect(()=>encodeBinary(NaN)).toThrow();expect(()=>encodeBinary(1n)).toThrow()
  const cycle:unknown[]=[];cycle.push(cycle);expect(()=>encodeBinary(cycle)).toThrow('nesting')
  const bad=new Uint8Array([77,71,86,49,5,255,255,255,255]);expect(()=>decodeBinary(bad)).toThrow('limit')
 })
 it('decodes hinted numeric triples into flat typed arrays and falls back on shape mismatch',()=>{
  const value={positions:[[0.5,1.5,-2.5],[3.5,4.5,5.5]],colors:[[0,128,255],[1,2,3]],triangles:[[0,1,2],[4294967295,0,128]],empty:[]}
  const decoded=decodeBinary(encodeBinary(value),{positions:'f64',colors:'u8',triangles:'u32',empty:'u32'}) as Record<string,unknown>
  expect(decoded.positions).toBeInstanceOf(Float64Array);expect([...decoded.positions as Float64Array]).toEqual([0.5,1.5,-2.5,3.5,4.5,5.5])
  expect(decoded.colors).toBeInstanceOf(Uint8Array);expect([...decoded.colors as Uint8Array]).toEqual([0,128,255,1,2,3])
  expect(decoded.triangles).toBeInstanceOf(Uint32Array);expect([...decoded.triangles as Uint32Array]).toEqual([0,1,2,4294967295,0,128])
  expect(decoded.empty).toBeInstanceOf(Uint32Array);expect(decoded.empty).toHaveLength(0)
  // Integer tags, ragged rows and out-of-range values keep the generic nested arrays.
  expect(decodeBinary(encodeBinary({positions:[[1,2,3]]}),{positions:'f64'})).toEqual({positions:[[1,2,3]]})
  expect(decodeBinary(encodeBinary({positions:[[1.5,2.5],[3.5,4.5,5.5]]}),{positions:'f64'})).toEqual({positions:[[1.5,2.5],[3.5,4.5,5.5]]})
  expect(decodeBinary(encodeBinary({colors:[[256,0,0]]}),{colors:'u8'})).toEqual({colors:[[256,0,0]]})
 })
 it('decodes stored, fixed and dynamic DEFLATE streams independently of zlib',()=>{
  let seed=17;const random=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed>>>24}
  for(const size of [0,1,2,17,256,4096,65537])for(const patterned of [true,false])for(const options of [{level:0},{strategy:constants.Z_FIXED},{level:9}]){
   const data=Uint8Array.from({length:size},(_,i)=>patterned?i%13:random()),compressed=deflateRawSync(data,options),header=Buffer.alloc(4);header.writeUInt32LE(size)
   const packed=Buffer.concat([header,compressed]);expect(unpackWasm(packed)).toEqual(data)
   expect(()=>unpackWasm(packed.subarray(0,packed.length-1))).toThrow()
  }
  expect(()=>unpackWasm(new Uint8Array([255,255,255,255]))).toThrow('limit')
 })
 it('executes commands and source compilation with JSON serialization disabled',()=>{
  const parse=vi.spyOn(JSON,'parse').mockImplementation(()=>{throw new Error('JSON parse forbidden')})
  const stringify=vi.spyOn(JSON,'stringify').mockImplementation(()=>{throw new Error('JSON stringify forbidden')})
  let id:number|undefined,result:unknown
  try{
   id=callGeometryRust<number>('cad',{action:'cube',size:[2,3,4],center:false})
   result=prepareGraphRust('text','show box([2,3,4])')
   callGeometryRust('cad',{action:'delete',ids:[id]});id=undefined
  }finally{parse.mockRestore();stringify.mockRestore();if(id!==undefined)callGeometryRust('cad',{action:'delete',ids:[id]})}
  expect(result).toMatchObject({ok:true})
 })
})
