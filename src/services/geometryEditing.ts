/** Shared parameters only; each kernel edits its own representation. */
export type GeometryDeformation =
 | {kind:'twist';origin:number[];radians_per_unit:number}
 | {kind:'bend';origin:number[];radius:number}
 | {kind:'lattice';min:number[];max:number[];controls:number[][]}
export interface GeometryBrush {center:number[];radius:number;displacement:number[]}
/** Radial weight profile over normalized distance; `smooth` is the legacy smoothstep. */
export type SculptFalloff='smooth'|'linear'|'sharp'|'root'|'sphere'|'constant'
export const SCULPT_FALLOFFS:readonly SculptFalloff[]=['smooth','linear','sharp','root','sphere','constant']
/** Mirror the stroke across axis-aligned planes through `origin` (default world origin). */
export interface SculptSymmetry {axes:[boolean,boolean,boolean];origin?:number[]}
export type SculptKind='grab'|'draw'|'inflate'|'smooth'|'flatten'|'pinch'
export const SCULPT_KINDS:readonly SculptKind[]=['grab','draw','inflate','smooth','flatten','pinch']
/**
 * Representation-independent sculpt stroke. `grab` translates by `displacement`;
 * `draw`/`inflate` push along the area/vertex normal by `strength` (a distance,
 * negative carves); `smooth`/`flatten`/`pinch` move by fraction `strength` in [0, 1].
 */
export type SculptBrush=({kind:'grab';displacement:number[]}|{kind:Exclude<SculptKind,'grab'>;strength:number})&{center:number[];radius:number;falloff?:SculptFalloff;symmetry?:SculptSymmetry}
const finiteCoord=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v)&&Math.abs(v)<=1e6
/** Client-side mirror of the kernel checks so UI errors are immediate and descriptive. */
export function validateSculptBrush(brush:SculptBrush):void {
 if(!Array.isArray(brush.center)||brush.center.length!==3||!brush.center.every(finiteCoord))throw new Error('Sculpt brush center must be a finite 3D point.')
 if(!finiteCoord(brush.radius)||brush.radius<=0)throw new Error('Sculpt brush radius must be positive.')
 if(brush.falloff!==undefined&&!SCULPT_FALLOFFS.includes(brush.falloff))throw new Error('Unknown sculpt falloff.')
 if(brush.symmetry){const o=brush.symmetry.origin;if(brush.symmetry.axes.length!==3||(o!==undefined&&(o.length!==3||!o.every(finiteCoord))))throw new Error('Invalid sculpt symmetry.')}
 if(brush.kind==='grab'){if(brush.displacement.length!==3||!brush.displacement.every(finiteCoord))throw new Error('Sculpt grab displacement must be a finite 3D vector.');return}
 if(!SCULPT_KINDS.includes(brush.kind))throw new Error('Unknown sculpt brush kind.')
 if(!Number.isFinite(brush.strength))throw new Error('Sculpt brush strength must be finite.')
 if((brush.kind==='smooth'||brush.kind==='flatten'||brush.kind==='pinch')&&(brush.strength<0||brush.strength>1))throw new Error(`Sculpt ${brush.kind} strength must be within [0, 1].`)
 if((brush.kind==='draw'||brush.kind==='inflate')&&Math.abs(brush.strength)>1e6)throw new Error('Sculpt brush strength exceeds coordinate limits.')
}
