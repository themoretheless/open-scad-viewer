import type {DirectBody} from './directModeling'
import {inspectPolygonMesh} from './polygonKernel'
import {cross3,dot3,unit3,type Vec3} from './directSketchGeometry'
export type SurfacePattern='ribs'|'grooves'|'knurl'|'fuzzy'|'dimples'|'waves'
export interface SurfaceTextureOptions {pattern:SurfacePattern;pitch:number;height:number;angle:number;seed:number;detail:number;invert:boolean;triangles?:number[];origin:Vec3;u:Vec3;v:Vec3}
const sub=(a:number[],b:number[])=>a.map((v,k)=>v-b[k]),mix=(a:number,b:number,t:number)=>a+(b-a)*t
function noise(p:number[],seed:number){const cell=p.map(Math.floor),f=p.map((v,k)=>{const t=v-cell[k];return t*t*(3-2*t)}),hash=(x:number,y:number,z:number)=>{let h=Math.imul(x,374761393)^Math.imul(y,668265263)^Math.imul(z,2147483647)^seed;h=Math.imul(h^(h>>>13),1274126177);return ((h^(h>>>16))>>>0)/4294967295};const layer=(z:number)=>mix(mix(hash(cell[0],cell[1],z),hash(cell[0]+1,cell[1],z),f[0]),mix(hash(cell[0],cell[1]+1,z),hash(cell[0]+1,cell[1]+1,z),f[0]),f[1]);return mix(layer(cell[2]),layer(cell[2]+1),f[2])}
export function textureHeight(p:number[],o:SurfaceTextureOptions){
 const q=sub(p,o.origin),a=o.angle*Math.PI/180,x=dot3(q,o.u)/o.pitch,y=dot3(q,o.v)/o.pitch,s=x*Math.cos(a)-y*Math.sin(a),t=x*Math.sin(a)+y*Math.cos(a),wave=(v:number)=>(1+Math.cos(v*2*Math.PI))/2
 let h=0
 switch(o.pattern){case 'ribs':h=wave(s)**2;break;case 'grooves':h=-(wave(s)**4);break;case 'knurl':h=(wave(s+t)*wave(s-t))**.5;break;case 'fuzzy':h=2*noise(q.map(v=>v/o.pitch*2),o.seed)-1;break;case 'dimples':{const r=Math.hypot(s-Math.round(s),t-Math.round(t))/.42;h=r<1?-((1-r*r)**2):0;break}case 'waves':h=wave(s+.25*Math.sin(t*2*Math.PI));break;default:throw Error('Unknown surface pattern.')}
 return h*o.height*(o.invert?-1:1)
}
/** Shared-edge refinement followed by deterministic geometric displacement. */
export function textureSurface(body:DirectBody,o:SurfaceTextureOptions):DirectBody{
 if(![o.pitch,o.height,o.angle,o.seed,o.detail,...o.origin,...o.u,...o.v].every(Number.isFinite)||o.pitch<=0||o.height<=0||o.height>o.pitch/4||!Number.isInteger(o.detail)||o.detail<3||o.detail>8||!Number.isInteger(o.seed))throw Error('Use positive pitch/height, height ≤ pitch/4, detail 3–8 and an integer seed.')
 if(Math.abs(dot3(o.u,o.v))>1e-6||Math.abs(dot3(o.u,o.u)-1)>1e-6||Math.abs(dot3(o.v,o.v)-1)>1e-6)throw Error('Invalid texture frame.')
 const original=inspectPolygonMesh(body.mesh);if(!original.closed||original.signedVolumeMm3<=0)throw Error('Texture requires a closed outward-oriented solid.')
 let points=Array.from({length:body.mesh.positions.length/3},(_,i)=>body.mesh.positions.slice(i*3,i*3+3)),faces=Array.from({length:body.mesh.indices.length/3},(_,i)=>body.mesh.indices.slice(i*3,i*3+3)),active=faces.map((_,i)=>o.triangles?o.triangles.includes(i):true)
 if(!active.some(Boolean)||o.triangles?.some(i=>!Number.isInteger(i)||i<0||i>=faces.length))throw Error('Choose a valid surface.')
 const boundary:number[][][]=[]
 if(o.triangles){const edges=new Map<string,{ids:number[];count:number}>();faces.forEach((f,i)=>{if(active[i])for(let k=0;k<3;k++){const ids=[f[k],f[(k+1)%3]].sort((a,b)=>a-b),key=ids.join(',');const e=edges.get(key)??{ids,count:0};e.count++;edges.set(key,e)}});for(const e of edges.values())if(e.count===1)boundary.push(e.ids.map(i=>points[i]))}
 let longest=0;for(const f of faces)for(let k=0;k<3;k++)longest=Math.max(longest,Math.hypot(...sub(points[f[k]],points[f[(k+1)%3]])))
 const levels=Math.max(0,Math.ceil(Math.log2(longest/(o.pitch/o.detail))))
 if(faces.length*4**levels>40000)throw Error('Texture exceeds 40000 triangles. Increase pitch, lower detail, or simplify the body.')
 for(let level=0;level<levels;level++){const cache=new Map<string,number>(),next:number[][]=[],flags:boolean[]=[];const midpoint=(a:number,b:number)=>{const key=[a,b].sort((a,b)=>a-b).join(',');let i=cache.get(key);if(i===undefined){i=points.length;points.push(points[a].map((v,k)=>(v+points[b][k])/2));cache.set(key,i)}return i};faces.forEach(([a,b,c],i)=>{const ab=midpoint(a,b),bc=midpoint(b,c),ca=midpoint(c,a);next.push([a,ab,ca],[ab,b,bc],[ca,bc,c],[ab,bc,ca]);flags.push(active[i],active[i],active[i],active[i])});faces=next;active=flags}
 const normals=points.map(()=>[0,0,0]),enabled=new Set<number>();faces.forEach((f,i)=>{const n=cross3(sub(points[f[1]],points[f[0]]),sub(points[f[2]],points[f[0]]));for(const id of f){for(let k=0;k<3;k++)normals[id][k]+=n[k];if(active[i])enabled.add(id)}})
 const displaced=points.map((p,i)=>{if(!enabled.has(i))return p;let fade=1;for(const [a,b] of boundary){const ab=sub(b,a),t=Math.max(0,Math.min(1,dot3(sub(p,a),ab)/dot3(ab,ab))),d=Math.hypot(...sub(p,a.map((v,k)=>v+t*ab[k])));fade=Math.min(fade,d/(o.pitch/2))}fade=fade*fade*(3-2*fade);const n=unit3(normals[i]),h=textureHeight(p,o)*fade;return h===0?p:p.map((v,k)=>Math.round((v+n[k]*h)*1e6)/1e6)})
 for(const f of faces){const old=cross3(sub(points[f[1]],points[f[0]]),sub(points[f[2]],points[f[0]])),next=cross3(sub(displaced[f[1]],displaced[f[0]]),sub(displaced[f[2]],displaced[f[0]]));if(dot3(old,next)<=0)throw Error('Texture folds a triangle. Reduce height or increase pitch.')}
 const mesh={positions:displaced.flat(),indices:faces.flat()},report=inspectPolygonMesh(mesh);if(!report.closed||report.signedVolumeMm3<=0)throw Error('Invalid textured surface.')
 return {...body,mesh}
}
