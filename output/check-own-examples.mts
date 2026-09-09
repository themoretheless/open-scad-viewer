import {EXAMPLES} from '../src/data/examples'
import {parseOpenSCAD} from '../src/services/openscadParser'
for(const [name,source]of Object.entries({sphereCut:'difference(){cube([10,10,10],center=true);sphere(r=3,$fn=48);}',...EXAMPLES})){const t=performance.now();try{const r=await parseOpenSCAD(source);console.log(name,Math.round(performance.now()-t),r.volume)}catch(e){console.log(name,String(e))}}
