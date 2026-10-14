import {readFileSync} from 'node:fs'
export type MiterLawMatrixMode={closed:boolean;affine:boolean;authored:boolean;guided:boolean}
const original=readFileSync(new URL('../examples/rush/miter-moving-frame-guide-affine-hollow-corrected.r',import.meta.url),'utf8')
const originalClosed=readFileSync(new URL('../examples/rush/closed-miter-frame-guide-affine-hollow.r',import.meta.url),'utf8')
export const miterLawMatrixModes:MiterLawMatrixMode[]=[false,true].flatMap(closed=>Array.from({length:8},(_,bits)=>({
 closed,affine:!!(bits&1),authored:!!(bits&2),guided:!!(bits&4),
})))
export function miterLawMatrixName(mode:MiterLawMatrixMode):string {
 const laws=[mode.affine?'affine':null,mode.authored?'authored':null,mode.guided?'guide':null].filter(Boolean).join('-')||'plain'
 return `miter-laws-${mode.closed?'closed':'open'}-${laws}`
}
export function miterLawMatrixSource(mode:MiterLawMatrixMode):string {
 let source=(mode.closed?originalClosed:original)
  .replace('max_deviation: 0.6mm','max_deviation: 2mm')
  .replace('cap_correction_tolerance: 0.6mm','cap_correction_tolerance: 2mm')
  .replace('values: [1,1], weights: [1,1]','values: [1,1.25], weights: [1,1]')
  .replace('values: [0deg,0deg]','values: [0deg,15deg]')
  .replace('values: [[0mm,0mm,0mm],[0mm,0mm,0mm]]','values: [[0.125mm,-0.25mm,0mm],[0.125mm,-0.25mm,0mm]]')
 if(mode.closed)source=source
  .replace(/^ scale:.*$/m,' scale: {degree: 2,knots: [0,0,0,1,1,1],values: [1,1.25,1],weights: [1,2,1]},')
  .replace(/^ twist:.*$/m,' twist: {degree: 2,knots: [0,0,0,1,1,1],values: [0deg,15deg,0deg],weights: [1,2,1]},')
  .replace(/^ retained_wall_max/m,' center_law: {degree: 1,knots: [23,23,29,29],values: [[0.125mm,-0.25mm,0mm],[0.125mm,-0.25mm,0mm]],weights: [1,1]},\n retained_wall_max')
 if(!mode.affine)source=source.replace(/^\s*(axis_scale|center_law):.*\n/gm,'')
 if(!mode.authored)source=source.replace(/^\s*(frame_axis|frame_normal):.*\n/gm,'')
 if(!mode.guided)source=source.replace(/^\s*orientation_guide:.*\n/gm,'').replace(/^rail = [\s\S]*?\)\n/m,'')
 return source
}
