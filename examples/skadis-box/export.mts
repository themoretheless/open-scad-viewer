import {readFileSync,writeFileSync} from 'node:fs';
import {executeIndependentOpenScad,independentOpenScadMetrics} from '../../src/mcp/independentOpenScadExecution.ts';
import {buildSTLBuffer} from '../../src/services/stlExport.ts';
const dir=new URL('./',import.meta.url);
const source=readFileSync(new URL('skadis-box.scad',dir),'utf8');
for(const [part,name] of [[1,'box'],[2,'backing-rail']] as const){
 const {result}=await executeIndependentOpenScad({source:source.replace('part = 0;',`part = ${part};`),files:[],quality:'full',time:0});
 writeFileSync(new URL(`${name}.stl`,dir),Buffer.from(buildSTLBuffer(result.meshes)));
 console.log(name,JSON.stringify(independentOpenScadMetrics(result)));
}
writeFileSync(new URL('viewer-url.txt',dir),'http://localhost:5187/#code='+Buffer.from(source).toString('base64url'));
