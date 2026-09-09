import {readFileSync,writeFileSync} from 'node:fs';
import {parseOpenSCAD} from '../../src/services/openscadParser.ts';
import {isGeometryEvaluationResultPayload} from '../../src/services/geometryWorkerProtocol.ts';
const source=readFileSync('examples/skadis-box/dovetail-preview.scad','utf8');
const r=await parseOpenSCAD(source,{quality:'full'});
console.log('valid',isGeometryEvaluationResultPayload(r),'meshes',r.meshes.length);
const url='http://127.0.0.1:5187/#code='+Buffer.from(source).toString('base64url');
writeFileSync('dist/skadis-dovetail.html','<!doctype html><script>location.replace('+JSON.stringify(url)+')</script>');
