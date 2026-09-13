import {callGeometryRust} from './geometry/kernel'
export interface ThreadOptions {
  diameter: number
  pitch: number
  length: number
  internal: boolean
  wall: number
  clearance: number
  starts: number
  left_handed: boolean
  segments_per_turn: number
}

export interface ThreadBuild {
 source:string
 mesh:{positions:number[];indices:number[]}
 report:{
  generator:string;profile:string;units:string;handedness:string;internal:boolean
  nominal_diameter_mm:number;pitch_mm:number;lead_mm:number;length_mm:number;starts:number
  radial_clearance_mm:number;clearance_convention:string;major_diameter_mm:number;minor_diameter_mm:number
  outer_diameter_mm:number;minimum_wall_mm:number|null;profile_depth_mm:number;flank_included_angle_degrees:number
  segments_per_turn:number;angular_columns:number;vertex_count:number;triangle_count:number
  tolerance_class:null;limitations:string[]
 }
}
export function threadRadiusAt(options:ThreadOptions,angleRadians:number,z:number):number {
 return callGeometryRust('cad_thread_radius',{options,angle:angleRadians,z})
}
export function buildModelGraphThread(options:ThreadOptions):ThreadBuild {
 return callGeometryRust('cad_thread_geometry',{options})
}
