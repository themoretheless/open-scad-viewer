import {it,expect} from 'vitest'
import {exportAssemblyFrames,frameArchive} from '../src/services/assemblyFrameExport'
import {emptyDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import type {AssemblyAnimation} from '../src/services/assemblyAnimation'
it('exports deterministic timeline frames with shared bounds and exact endpoint timing',()=>{
 const doc=emptyDirectDocument(),brep=createBrepBox([0,0,0],[2,4,6]);doc.bodies=[{id:'p',name:'P',brep,mesh:tessellateNurbsBrep(brep,1)}]
 const clip:AssemblyAnimation={version:1,frames:[{time:1,poses:[{id:'p',translation:[0,0,0],angle:0,rotation:[0,0,0],scale:1}],camera:{yaw:0,pitch:.3}},{time:2.25,poses:[{id:'p',translation:[3,0,0],angle:0,rotation:[90,0,90],scale:1}],camera:{yaw:.2,pitch:.3}}]}
 const before=serializeDirectDocument(doc),result=exportAssemblyFrames(doc,clip,4)
 expect(result.frames.map(f=>f.time)).toEqual([1,1.25,1.5,1.75,2,2.25]);expect(result.fps).toBe(4)
 expect(new Set(result.frames.map(f=>f.svg.match(/viewBox="([^"]+)"/)![1])).size).toBe(1)
 expect(result.frames[0].svg).not.toBe(result.frames.at(-1)!.svg);expect(exportAssemblyFrames(doc,clip,4)).toEqual(result);expect(serializeDirectDocument(doc)).toBe(before)
 clip.frames[1].time=120;expect(()=>exportAssemblyFrames(doc,clip,30)).toThrow(/300/)
})
it('writes ZIP offsets, checksums and refuses unsafe filenames or oversized exports',()=>{
 const bytes=new TextEncoder().encode('abc'),zip=frameArchive([{name:'frame-00001.png',bytes},{name:'manifest.json',bytes:new Uint8Array([1,2])}]),v=new DataView(zip.buffer)
 expect(v.getUint32(0,true)).toBe(0x04034b50);expect(v.getUint32(14,true)).toBe(0x352441c2)
 const end=zip.length-22;expect(v.getUint32(end,true)).toBe(0x06054b50);expect(v.getUint16(end+8,true)).toBe(2)
 const central=v.getUint32(end+16,true);expect(v.getUint32(central,true)).toBe(0x02014b50);expect(v.getUint32(central+42,true)).toBe(0)
 expect(()=>frameArchive([{name:'../outside',bytes}])).toThrow(/filename/)
 expect(()=>frameArchive([{name:'big',bytes:new Uint8Array(64*1024*1024+1)}])).toThrow(/64 MiB/)
})
