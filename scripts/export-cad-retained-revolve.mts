import 'fake-indexeddb/auto'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {authorBrepProfile,transformBrepProfile} from '../src/services/geometry/brepProfile'
import {withRetainedProfile} from '../src/services/retainedSketchProfile'
import {applySolidRevolve,type SolidRevolveOptions} from '../src/services/solidRevolve'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
await warmGeometryKernel()
const directory=resolve(process.argv[2]??'/tmp/cad-retained-revolve');mkdirSync(directory,{recursive:true})
const options:SolidRevolveOptions={sketchId:'profile',geometry:'exact',operation:'new',targetId:'',id:'result',name:'Revolved',tessellation:2,axis:'y',offset:0,angle:360,segments:32}
const circle=transformBrepProfile(authorBrepProfile({kind:'circle',radius:1}),[1,0,0,0,0,1,0,0,0,0,1,0,5,0,0,1])
const holed=authorBrepProfile({kind:'polygon',rings:[[[3,0],[6,0],[6,4],[3,4]],[[4,1],[4,3],[5,3],[5,1]]]})
const disconnected=authorBrepProfile({kind:'polygon',rings:[[[3,0],[4,0],[4,1],[3,1]],[[5,2],[6,2],[6,3],[5,3]]]})
const manifest={schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts:[] as object[]}
for(const fixture of [
 {name:'rational-torus',profile:circle,angle:360,volume:10*Math.PI**2,bounds:[[-6,-1,-6],[6,1,6]]},
 {name:'holed-full',profile:holed,angle:360,volume:90*Math.PI,bounds:[[-6,0,-6],[6,4,6]]},
 {name:'disconnected-full',profile:disconnected,angle:360,volume:18*Math.PI,bounds:[[-6,0,-6],[6,3,6]],solids:2},
 {name:'holed-quarter',profile:holed,angle:90,volume:22.5*Math.PI,bounds:[[0,0,-6],[6,4,0]]},
]){
 const sketch=withRetainedProfile({id:'profile',name:fixture.name,closed:true,points:[]},fixture.profile)
 writeFileSync(resolve(directory,fixture.name+'-project.json'),JSON.stringify({version:1,bodies:[],sketches:[sketch]},null,2)+'\n')
 const result=applySolidRevolve({version:1,bodies:[],sketches:[sketch]},{...options,angle:fixture.angle})
 console.log('Exporting',fixture.name)
 const step=await exportSolidStepCurrent(result.bodies[0]),file=fixture.name+'.step'
 writeFileSync(resolve(directory,file),step)
 manifest.parts.push({name:fixture.name,file,sha256:createHash('sha256').update(step).digest('hex'),expected:{volumeMm3:fixture.volume,boundsMm:fixture.bounds,solids:'solids' in fixture?fixture.solids:1}})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify(manifest,null,2)+'\n')
