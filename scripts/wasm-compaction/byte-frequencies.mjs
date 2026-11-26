import binaryen from 'binaryen'
import {Buffer} from 'node:buffer'
import {appendEntry, readUnsigned, sections, unsigned} from './binary-sections.mjs'

// Compress the byte frequencies of each already-packed segment independently.
// Decoding runs once before the original start. All scratch memory is restored.
function huffmanChunk(bytes) {
  const counts=new Uint32Array(256)
  for (const byte of bytes) counts[byte]++
  const queue=Array.from(counts,(weight,id)=>({id,weight})).filter(node=>node.weight)
  if (queue.length<2) return
  const children=[]
  while(queue.length>1){
    queue.sort((a,b)=>b.weight-a.weight || b.id-a.id)
    const left=queue.pop(),right=queue.pop()
    const id=256+children.length
    children.push([left.id,right.id])
    queue.push({id,weight:left.weight+right.weight})
  }
  const root=queue[0].id
  const codes=[]
  function assign(id,bits,length){
    if(id<256){codes[id]={bits,length};return}
    const [left,right]=children[id-256]
    assign(left,bits*2,length+1)
    assign(right,bits*2+1,length+1)
  }
  assign(root,0,0)
  // Each chunk contains at most 32768 symbols, bounding code depth below 32.
  const bitCount=bytes.reduce((sum,byte)=>sum+codes[byte].length,0)
  const flagsBytes=Math.ceil(children.length*2/8)
  const tableBytes=flagsBytes+children.length+1
  const size=tableBytes+Math.ceil(bitCount/8)
  if(size+32>=bytes.length) return
  const data=Buffer.alloc(size)
  // Breadth-first internal children have implicit successive indices.
  // Only a branch bitmap and the literal leaves need to travel in the module.
  const pending=[root]
  let leaf=flagsBytes,slot=0
  for (let node=0;node<pending.length;node++) {
    for (const child of children[pending[node]-256]) {
      if (child>=256) { data[slot>>>3]|=1<<(slot&7);pending.push(child) }
      else data[leaf++]=child
      slot++
    }
  }
  let cursor=tableBytes*8
  for(const byte of bytes){
    const {bits,length}=codes[byte]
    for(let n=length-1;n>=0;n--,cursor++) data[cursor>>>3]|=((bits>>>n)&1)<<(7-(cursor&7))
  }
  return {data,internalCount:children.length,tableBytes}
}

export function compactWasmByteFrequencies(input) {
  const original=Buffer.from(input.buffer,input.byteOffset,input.byteLength)
  const list=sections(original),section=id=>list.find(s=>s.id===id)
  if(section(2)||!section(8)||section(5)?.data[0]!==1) return original
  const data=section(11).data,cursor={at:0},count=readUnsigned(data,cursor)
  const encoded=[unsigned(count)],calls=[]
  let scratch=0,end=0
  for(let segment=0;segment<count;segment++){
    const start=cursor.at,tag=readUnsigned(data,cursor)
    if(tag===1 && readUnsigned(data,cursor)===0){encoded.push(data.subarray(start,cursor.at));continue}
    if(tag!==0||data[cursor.at++]!==65)return original
    const base=readUnsigned(data,cursor)
    if(data[cursor.at++]!==11)return original
    const headerEnd=cursor.at,length=readUnsigned(data,cursor)
    if(!length){encoded.push(data.subarray(start,cursor.at));continue}
    if(base<65536||base<end||base+length>=0x40000000||cursor.at+length>data.length)return original
    end=base+length
    const pieces=[],reverse=[]
    let packed=0
    // Align entropy models to local data changes. Include initializer overhead
    // in the split decision so many tiny blocks cannot inflate the module.
    const stride=4096, blocks=Math.ceil(length/stride)
    const costs=new Float64Array(blocks+1), choices=[]
    for(let i=blocks-1;i>=0;i--){
      let best=Infinity
      for(let stop=i+1;stop<=Math.min(i+8,blocks);stop++){
        const at=i*stride, end=Math.min(stop*stride,length)
        const raw=data.subarray(cursor.at+at,cursor.at+end)
        const h=huffmanChunk(raw)
        const cost=(h?.data.length??raw.length)+(h?36:15)+costs[stop]
        if(cost<best){best=cost;choices[i]={stop,raw,h}}
      }
      costs[i]=best
    }
    for(let i=0;i<blocks;){
      const at=i*stride
      const {stop,raw,h}=choices[i],chunk=h?.data??raw
      i=stop
      if(h) scratch=Math.max(scratch,chunk.length)
      if(h||at!==packed) reverse.push({src:base+packed,dst:base+at,length:raw.length,packed:chunk.length,h})
      pieces.push(chunk)
      packed+=chunk.length
    }
    encoded.push(data.subarray(start,headerEnd),unsigned(packed),...pieces)
    calls.push(...reverse.reverse())
    cursor.at+=length
  }
  if(!scratch||cursor.at!==data.length)return original
  const decoder=binaryen.parseText(`(module (memory 1)
    (func (param $src i32)(param $dst i32)(param $length i32)(param $root i32)
      (local $node i32)(local $bits i32)(local $left i32)
      (local $slot i32)(local $next i32)(local $leaf i32)(local $child i32)
      (local.set $leaf (i32.shr_u (i32.add (i32.shl (local.get $root)(i32.const 1))(i32.const 7))(i32.const 3)))
      (local.set $next (i32.const 257))
      (loop $tree
        (if (i32.and (i32.load8_u (i32.shr_u (local.get $slot)(i32.const 3)))
                     (i32.shl (i32.const 1)(i32.and (local.get $slot)(i32.const 7))))
          (then (local.set $child (local.get $next))
                (local.set $next (i32.add (local.get $next)(i32.const 1))))
          (else (local.set $child (i32.load8_u (local.get $leaf)))
                (local.set $leaf (i32.add (local.get $leaf)(i32.const 1)))))
        (i32.store16 offset=32768 (i32.shl (local.get $slot)(i32.const 1))(local.get $child))
        (br_if $tree (i32.lt_u (local.tee $slot (i32.add (local.get $slot)(i32.const 1)))
                               (i32.shl (local.get $root)(i32.const 1)))))
      (loop $byte
        (local.set $node (i32.const 256))
        (loop $branch
          (if (i32.eqz (local.get $left)) (then
            (local.set $bits (i32.load8_u (local.get $src)))
            (local.set $src (i32.add (local.get $src)(i32.const 1)))
            (local.set $left (i32.const 8))))
          (local.set $left (i32.sub (local.get $left)(i32.const 1)))
          (local.set $node (i32.load16_u offset=32768 (i32.add
            (i32.shl (i32.sub (local.get $node)(i32.const 256))(i32.const 2))
            (i32.shl (i32.and (i32.shr_u (local.get $bits)(local.get $left))(i32.const 1))(i32.const 1)))))
          (br_if $branch (i32.ge_u (local.get $node)(i32.const 256))))
        (i32.store8 (local.get $dst)(local.get $node))
        (local.set $dst (i32.add (local.get $dst)(i32.const 1)))
        (br_if $byte (local.tee $length (i32.sub (local.get $length)(i32.const 1)))))))`)
  try{
    if(!decoder.validate())throw new Error('Invalid byte frequency decoder')
    const bodies=sections(Buffer.from(decoder.emitBinary())).find(s=>s.id===10).data
    const oldStart=readUnsigned(section(8).data,{at:0})
    const index=readUnsigned(section(3).data,{at:0})
    const type=readUnsigned(section(1).data,{at:0})
    section(1).data=appendEntry(section(1).data,Buffer.from([0x60,4,127,127,127,127,0]))
    section(1).data=appendEntry(section(1).data,Buffer.from([0x60,0,0]))
    section(3).data=appendEntry(appendEntry(section(3).data,unsigned(type)),unsigned(type+1))
    const start=[0]
    function constant(n){const b=[...unsigned(n)];if(b.at(-1)&64){b[b.length-1]|=128;b.push(0)}start.push(65,...b)}
    for(const c of calls){
      constant(c.h?0:c.dst);constant(c.src);constant(c.packed);start.push(252,10,0,0)
      if(c.h){
        constant(c.h.tableBytes);constant(c.dst);constant(c.length);constant(c.h.internalCount)
        start.push(16,...unsigned(index))
      }
    }
    constant(32768);constant(0);constant(1020);start.push(252,11,0)
    constant(0);constant(0);constant(scratch);start.push(252,11,0,16,...unsigned(oldStart),11)
    section(10).data=appendEntry(appendEntry(section(10).data,bodies.subarray(1)),Buffer.concat([unsigned(start.length),Buffer.from(start)]))
    section(8).data=unsigned(index+1)
    section(11).data=Buffer.concat(encoded)
    const result=Buffer.concat([original.subarray(0,8),...list.flatMap(s=>[Buffer.from([s.id]),unsigned(s.data.length),s.data])])
    if(result.length>=original.length)return original
    const initial=bytes=>Buffer.from(new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports.memory.buffer)
    if(!initial(original).equals(initial(result)))throw new Error('Byte frequency decoder changed initial geometry memory')
    return result
  }finally{decoder.dispose()}
}
