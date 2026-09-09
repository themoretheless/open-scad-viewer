import {readFileSync,writeFileSync} from 'node:fs';
import {parseOpenSCAD} from '../../src/services/openscadParser.ts';
import {buildSTLBuffer} from '../../src/services/stlExport.ts';
import {isGeometryEvaluationResultPayload} from '../../src/services/geometryWorkerProtocol.ts';
const dir=new URL('./',import.meta.url);
const result=await parseOpenSCAD(readFileSync(new URL('skadis-integral.scad',dir),'utf8'),{quality:'full'});
let source='// SKADIS hook assembly preview. Editable: skadis-integral.scad\n';
for(const [i,m] of result.meshes.entries()) {
 const buf=Buffer.from(buildSTLBuffer([m])); const points:number[][]=[]; const faces:number[][]=[]; const ids=new Map<string,number>();
 for(let t=0;t<buf.readUInt32LE(80);t++) {
  const face:number[]=[];
  for(let v=0;v<3;v++) {
   const p=[0,1,2].map(k=>buf.readFloatLE(84+t*50+12+v*12+k*4));
   const key=p.join(','); if(!ids.has(key)){ids.set(key,points.length); points.push(p);} face.push(ids.get(key)!);
  }
  faces.push(face.reverse());
 }
 source+=`color(${JSON.stringify(i===0?[0.18,0.55,0.64]:[0.95,0.65,0.22])}) polyhedron(points=${JSON.stringify(points)},faces=${JSON.stringify(faces)});\n`;
}
writeFileSync(new URL('integral-preview.scad',dir),source);
const preview=await parseOpenSCAD(source,{quality:'full'});
console.log(JSON.stringify({valid:isGeometryEvaluationResultPayload(preview),volume:preview.volume,originalVolume:result.volume,topology:preview.meshes.map(m=>m.topology)}));
const url='http://127.0.0.1:5187/#code='+Buffer.from(source).toString('base64url');
writeFileSync(new URL('../../dist/skadis-integral.html',import.meta.url),'<!doctype html><script>location.replace('+JSON.stringify(url)+')</script>');
