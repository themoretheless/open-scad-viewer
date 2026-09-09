import {readFileSync} from 'node:fs';
import {executeIndependentOpenScad} from '../../src/mcp/independentOpenScadExecution.ts';
const base=readFileSync('examples/skadis-box/skadis-dovetail.scad','utf8');
for(const config of [{},{width:72,depth:35,height:50,hook_spacing_steps:1},{width:220,depth:110,height:150,hook_count:3},{width:60}]){
 let source=base;
 for(const [key,value] of Object.entries(config))source=source.replace(new RegExp(key+' = [0-9.]+;'),key+' = '+value+';');
 for(const part of [1,2]){
 try{
 const {result}=await executeIndependentOpenScad({source:source.replace('part = 0;', 'part = '+part+';'),files:[],quality:'full',time:0});
 if(config.width===60)throw Error('Invalid configuration was accepted');
 if(result.meshes.length!==1 || !(result.volume>0))throw Error('Invalid solid');
 console.log(JSON.stringify({config,part,volume:result.volume}));
 }catch(e){if(config.width===60 && !String(e).includes('was accepted'))console.log('Invalid width rejected');else throw e;}
 }
}
