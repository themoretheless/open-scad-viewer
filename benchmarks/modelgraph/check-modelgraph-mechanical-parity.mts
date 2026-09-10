/** Run after building a native mechanical-check harness; compares coordinates, topology and reports. */
import {spawnSync} from 'node:child_process'
import {writeFileSync} from 'node:fs'
import {buildModelGraphGear} from '../../src/services/modelGraphGears'
import {buildModelGraphThread} from '../../src/services/modelGraphThreads'
import {buildModelGraphPlanetary} from '../../src/services/modelGraphPlanetary'
import {GEAR_DEFAULTS,THREAD_DEFAULTS,PLANETARY_DEFAULTS} from '../../src/services/mechanicalGeneratorContract'
const functions:any={gear:buildModelGraphGear,thread:buildModelGraphThread,planetary:buildModelGraphPlanetary}
const cases:any[]=[]; const add=(kind:string,base:any,variants:any[])=>variants.forEach((v,i)=>cases.push({name:kind+'_'+i,kind,options:{...base,...v}}))
add('gear',GEAR_DEFAULTS,[{}, {teeth:18}, {teeth:64}, {teeth:128}, {module:0.2,bore:0,clearance:0.02,backlash:0.01}, {module:7}, {bore:0}, {flank_segments:3}, {flank_segments:12}, {pressure_angle:14.5,teeth:40}, {pressure_angle:30}, {internal:true,bore:0,teeth:72}, {internal:true,bore:0,teeth:128}, {teeth:8}, {teeth:129}, {teeth:24.5}, {flank_segments:2}, {module:0}, {pressure_angle:45}, {thickness:0}, {backlash:2}, {clearance:4}, {bore:1000}, {internal:true}, {internal:true,bore:0,teeth:72,rim_width:0}])
add('thread',THREAD_DEFAULTS,[{}, {internal:true}, {left_handed:true}, {internal:true,left_handed:true}, {starts:2,length:3}, {starts:4,length:3}, {segments_per_turn:16,length:3}, {segments_per_turn:96,length:1.5}, {length:0.375}, {pitch:0.7,length:1.234,diameter:6}, {length:96}, {length:97}, {diameter:2}, {pitch:0}, {clearance:10}, {starts:5}, {starts:1.5}, {segments_per_turn:15}, {starts:4,segments_per_turn:16}])
add('planetary',PLANETARY_DEFAULTS,[{}, {carrier_angle:45}, {carrier_angle:-120}, {sun_teeth:25,planet_teeth:23,carrier_angle:17.5}, {planet_count:2}, {planet_count:4}, {planet_count:5}, {planet_count:7}, {sun_teeth:18,planet_teeth:18}, {sun_teeth:8}, {carrier_angle:360001}, {flank_segments:3}])
const binary=process.argv[2]??'/private/tmp/modelgraph-schema-check/target/debug/mechanical-check'
const run=spawnSync(binary,[],{input:cases.map(x=>JSON.stringify(x)).join('\n')+'\n',encoding:'utf8',maxBuffer:64*1024*1024})
if(run.status!==0)throw new Error(run.stderr||String(run.error))
const outputs=run.stdout.trim().split('\n').map(x=>JSON.parse(x));let maxCoordinateDelta=0;let coordinateCount=0;const failures:string[]=[]
const compare=(a:any,b:any,path:string,coordinates=false):void=>{
 if(typeof a==='number'&&typeof b==='number'){const delta=Math.abs(a-b);if(coordinates){maxCoordinateDelta=Math.max(maxCoordinateDelta,delta);coordinateCount++}if(delta>(coordinates?2e-7:1e-10*Math.max(1,Math.abs(a))))throw new Error(path+': '+a+' != '+b);return}
 if(a===null||b===null||typeof a!=='object'||typeof b!=='object'){if(a!==b)throw new Error(path+': '+a+' != '+b);return}
 const ka=Object.keys(a),kb=Object.keys(b);if(ka.join('|')!==kb.sort((a,b)=>ka.indexOf(a)-ka.indexOf(b)).join('|'))throw new Error(path+': keys differ '+ka+' vs '+kb)
 for(const k of ka)compare(a[k],b[k],path+'/'+k,coordinates)
}
function sourceArrays(source:string){const arrays:any[]=[];for(const m of source.matchAll(/(points|paths|faces)=/g)){let depth=0,end=m.index!+m[0].length;const start=end;for(;end<source.length;end++){if(source[end]==='[')depth++;if(source[end]===']'&&--depth===0){end++;break}}arrays.push({kind:m[1],value:JSON.parse(source.slice(start,end))})}return arrays}
for(let i=0;i<cases.length;i++){const c=cases[i],actual=outputs[i];let expected:any;try{expected=functions[c.kind](c.options)}catch(e){expected={error:(e as Error).message}}try{if(expected.error||actual.error){compare(expected.error,actual.error,c.name+'/error');continue}compare(expected.report,actual.report,c.name+'/report');const a=sourceArrays(expected.source),b=sourceArrays(actual.source);if(a.length!==b.length)throw new Error('source array counts differ');for(let j=0;j<a.length;j++){compare(a[j].kind,b[j].kind,'kind');compare(a[j].value,b[j].value,c.name+'/'+j,a[j].kind==='points')} }catch(e){failures.push((e as Error).message)}}
const fixture=cases.map(c=>{try{const expected=functions[c.kind](c.options);return {...c,report:expected.report,arrays:sourceArrays(expected.source).map(({kind,value})=>({kind,length:value.length,samples:[...new Set(Array.from({length:16},(_,i)=>Math.floor((value.length-1)*i/15)))].map(index=>({index,value:value[index]}))}))}}catch(e){return {...c,error:(e as Error).message}}});
writeFileSync('crates/modelgraph-runtime/tests/fixtures/mechanical-parity.json',JSON.stringify({description:'Reports, exact topology samples and coordinate samples captured from unchanged TypeScript mechanical generators. The differential script checks all coordinates and all topology.',cases:fixture})+'\n');
const report={cases:cases.length,coordinateCount,maxCoordinateDelta,failures};writeFileSync('output/modelgraph-mechanical-parity.json',JSON.stringify(report,null,2)+'\n');console.log(report);if(failures.length)process.exitCode=1
