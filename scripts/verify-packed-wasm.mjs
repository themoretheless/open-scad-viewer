import {brotliDecompressSync, inflateRawSync} from 'node:zlib'
import {decodeBase91} from './wasm-base91.mjs'
import {decodeBase85} from './wasm-base85.mjs'
import {createHash} from 'node:crypto'

export function verifyRawWasm(raw, expected, label) {
  if (!Buffer.from(raw.buffer, raw.byteOffset, raw.byteLength).equals(expected)) {
    throw new Error(`${label}: streaming WASM differs from the original`)
  }
  return raw.byteLength
}

/** Inspect emitted strings: source maps do not attribute inlined payload copies. */
export function verifyUniquePackedWasm(assets) {
  const owners = new Map()
  for (const {path, source} of assets) {
    for (const match of source.matchAll(/(["'`])(b85:[^"'`\\\r\n]+|b(?:91|9v):[^'`\\\r\n]+|b(?:9[2345]|Ax|Bx):[^'\\\r\n]+)\1/g)) {
      const hash = createHash('sha256').update(match[2]).digest('hex')
      if (owners.has(hash)) throw new Error(`Duplicate packed WASM literal: ${owners.get(hash)} and ${path}`)
      owners.set(hash, path)
    }
  }
  return owners.size
}

/** Validate the emitted literal without executing generated application code. */
export function verifyPackedWasmChunk(source, expected, label, compression = 'brotli') {
  if (compression !== 'brotli' && compression !== 'deflate') throw new Error(`${label}: unsupported packing format`)
  const literals = [...source.matchAll(/(["'`])(b85:[^"'`\\\r\n]+|b(?:91|9v):[^'`\\\r\n]+|b(?:9[2345]|Ax|Bx):[^'\\\r\n]+|[A-Za-z0-9+/]{64,}={0,2})\1/g)]
  if (literals.length !== 1) throw new Error(`${label}: expected exactly one packed WASM literal`)
  const literal = literals[0][2]
  const packed = (literal.startsWith('b91:')||literal.startsWith('b9v:')||literal.startsWith('b92:')||literal.startsWith('b93:')||literal.startsWith('b94:')||literal.startsWith('b95:')||literal.startsWith('bAx:')||literal.startsWith('bBx:')) ? Buffer.from(decodeBase91(literal)) : literal.startsWith('b85:') ? Buffer.from(decodeBase85(literal)) : Buffer.from(literal, 'base64')
  if (!literal.startsWith('b85:') && !(literal.startsWith('b91:')||literal.startsWith('b9v:')||literal.startsWith('b92:')||literal.startsWith('b93:')||literal.startsWith('b94:')||literal.startsWith('b95:')||literal.startsWith('bAx:')||literal.startsWith('bBx:')) && packed.toString('base64') !== literal) throw new Error(`${label}: invalid base64`)
  if (packed.length <= 4 || packed.length - 4 > 4 * 1024 * 1024) throw new Error(`${label}: compressed size limit`)
  const word = packed.readUInt32LE(0), multiple = !!(word & 0x80000000), size = word & 0x7fffffff
  if (size > 16 * 1024 * 1024 || size !== expected.length) throw new Error(`${label}: decoded size mismatch`)
  const streams=[]
  if(multiple){
    const count=packed[4]
    if(compression!=='brotli'||!count||count>8||packed.length<=5+8*count)throw new Error(`${label}: invalid stream table`)
    let at=5+8*count,total=0
    for(let i=0;i<count;i++){
      const length=packed.readUInt32LE(5+8*i),compressed=packed.readUInt32LE(9+8*i)
      total+=length
      if(!length||!compressed||total>size||at+compressed>packed.length)throw new Error(`${label}: invalid stream size`)
      streams.push({size:length,bytes:packed.subarray(at,at+compressed)});at+=compressed
    }
    if(total!==size||at!==packed.length)throw new Error(`${label}: package size mismatch`)
  }else streams.push({size,bytes:packed.subarray(4)})
  const unpack = compression === 'brotli' ? brotliDecompressSync : inflateRawSync
  const decoded=Buffer.concat(streams.map(stream=>{
    const {buffer,engine}=unpack(stream.bytes,{maxOutputLength:16*1024*1024,info:true})
    if(engine.bytesWritten!==stream.bytes.length)throw new Error(`${label}: trailing compressed data`)
    if(buffer.length!==stream.size)throw new Error(`${label}: decoded size mismatch`)
    return buffer
  }))
  if (!decoded.subarray(0, 8).equals(Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]))) throw new Error(`${label}: invalid WASM header`)
  new WebAssembly.Module(decoded)
  if (!decoded.equals(expected)) throw new Error(`${label}: emitted WASM differs from the original`)
  return decoded.length
}
