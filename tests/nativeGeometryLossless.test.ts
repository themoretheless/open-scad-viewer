import {expect,it} from 'vitest'
import {createNativeGeometryArtifact,isNativeGeometryArtifact,MAX_NATIVE_GEOMETRY_CHARACTERS} from '../src/core/nativeGeometry'
import {packNativeGeometrySnapshot,readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'

it('retains the stored snapshot ceiling and exact data through native lossless transport',()=>{
 const data={geometry:{coordinates:[1.0000000000000002,Number.MIN_VALUE,Number.MAX_VALUE]},padding:'a'.repeat(MAX_NATIVE_GEOMETRY_CHARACTERS+1024)}
 const document={nodes:[],root:'source'},before=JSON.stringify(data)
 expect(()=>createNativeGeometryArtifact('source','brep',data,document)).toThrow(/bounded transport/)
 const snapshot=createNativeGeometryArtifact('source','brep',data,document,packNativeGeometrySnapshot)
 expect(snapshot.geometryJson.length+snapshot.documentJson.length).toBeLessThanOrEqual(MAX_NATIVE_GEOMETRY_CHARACTERS)
 expect(isNativeGeometryArtifact(snapshot)).toBe(true)
 expect(JSON.stringify(readNativeGeometryData(snapshot))).toBe(before)
 expect(readNativeGeometryData(snapshot).geometry.coordinates).toEqual(data.geometry.coordinates)
 expect(Object.isFrozen(snapshot)).toBe(true)
})

it('refuses an altered snapshot and a hash-valid malformed compressed carrier',()=>{
 const packet=packNativeGeometrySnapshot('{"geometry":{"coordinates":[1,2,3]}}') as Record<string,unknown>
 const snapshot=createNativeGeometryArtifact('source','brep',packet,{})
 expect(()=>readNativeGeometryData({...snapshot,geometryJson:'{}'})).toThrow(/Invalid native geometry snapshot/)
 const malformed=createNativeGeometryArtifact('source','brep',{...packet,decodedBytes:1},{})
 expect(isNativeGeometryArtifact(malformed)).toBe(true)
 expect(()=>readNativeGeometryData(malformed)).toThrow(/stream or length mismatch/)
})
