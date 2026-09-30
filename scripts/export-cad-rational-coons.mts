import {writeFileSync} from 'node:fs'
import {emptyDirectDocument} from '../src/services/directModeling'
import {insertNurbsKnot} from '../src/services/nurbsCurve'
import {prepareCoonsBoundaryWeights,coonsNurbsPatch} from '../src/services/nurbsConstructors'
import {constructSolidSurface} from '../src/services/solidSurfaceConstruction'
const d=emptyDirectDocument(),points=[[[0,0,0],[1,0,1],[2,0,0]],[[0,2,0],[2,2,0]],[[0,0,0],[0,2,0]],[[2,0,0],[2,1,2],[2,2,0]]],weights=[[1,.8,2],[3,4],[1,3],[2,1.5,4]],domains=[[-3,7],[11,13],[.2,.9],[-20,-10]]
d.curves=points.map((controlPoints,i)=>({id:`boundary-${i}`,name:`Boundary ${i}`,curve:{degree:controlPoints.length-1,controlPoints,weights:weights[i]!,knots:[...Array(controlPoints.length).fill(0),...Array(controlPoints.length).fill(1)]}}))
if(process.argv.includes('--nonbinary'))d.curves.forEach((c,edge)=>{c.curve.weights=c.curve.weights.map((_,i)=>.1+(7+edge*13+i*17)/31)})
d.curves[0]!.curve=insertNurbsKnot(insertNurbsKnot(d.curves[0]!.curve,.17),.61)
d.curves[3]!.curve=insertNurbsKnot(d.curves[3]!.curve,.42)
d.curves.forEach((c,i)=>{const [a,b]=domains[i]!;c.curve.knots=c.curve.knots.map(k=>a!+(b!-a!)*k)})
if(process.argv.includes('--prepare')){
 if(!process.argv.includes('--nonbinary'))d.curves[0]!.curve.weights[d.curves[0]!.curve.weights.length-1]=.5
 const preparation=d.curves.map(c=>prepareCoonsBoundaryWeights(c.curve,1e-6))
 if(preparation.some(p=>!p.report.accepted))throw Error('Boundary preparation refused')
 d.surfaces!.push({id:'prepared-coons',name:'Prepared Coons',surface:coonsNurbsPatch(preparation.map(p=>p.curve)),segmentsU:24,segmentsV:24})
 writeFileSync((process.argv[2]??'/tmp/cad-rational-coons.json')+'.preparation.json',JSON.stringify(preparation))
}else d.surfaces!.push(constructSolidSurface(d,d.curves.map(c=>c.id),'nurbs-patch','mixed-coons'))
writeFileSync(process.argv[2]??'/tmp/cad-rational-coons.json',JSON.stringify(d))
