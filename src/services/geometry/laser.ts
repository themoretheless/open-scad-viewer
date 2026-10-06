import {callGeometryRust} from './kernel'

export type LaserPowerMode='m3'|'m4'
export type LaserOperationKind='line'|'fill'
export type LaserKerfMode='center'|'part'|'cavity'
export type LaserPathOrder='preserve'|'nearest'|'inner-first'|'inner-first-nearest'

export interface LaserMachineProfile {
  widthMm:number
  heightMm:number
  maxPower:number
  estimatedRapidMmMin:number
  powerMode:LaserPowerMode
  laserModeConfirmed:boolean
  supportsAirAssist:boolean
  flipY:boolean
  returnToOrigin:boolean
}

export interface LaserPath {
  points:readonly (readonly [number,number])[]
  closed:boolean
}

export interface LaserOperation {
  name:string
  kind:LaserOperationKind
  output:boolean
  speedMmMin:number
  power:number
  passes:number
  airAssist:boolean
  kerfMm:number
  kerfMode:LaserKerfMode
  pathOrder:LaserPathOrder
  paths:readonly LaserPath[]
}

export interface LaserPlan {
  machine:LaserMachineProfile
  operations:readonly LaserOperation[]
}

export interface LaserSummary {
  operationCount:number
  pathCount:number
  segmentCount:number
  cutDistanceMm:number
  rapidDistanceMm:number
  estimatedTimeS:number
  bounds:{min:[number,number];max:[number,number]}
}

export interface LaserProgram {
  dialect:'open-scad-viewer/laser-grbl 1'|'open-scad-viewer/laser-frame 1'
  gcode:string
  summary:LaserSummary
}

export interface LaserPreviewOperation {
  name:string
  kind:LaserOperationKind
  paths:readonly LaserPath[]
}

export interface LaserPreview {
  summary:LaserSummary
  operations:readonly LaserPreviewOperation[]
}

export const previewLaserPlan=(plan:LaserPlan):LaserPreview=>
  callGeometryRust<LaserPreview>('laser_preflight',plan)

export const previewLaserFrame=(plan:LaserPlan):LaserPreview=>
  callGeometryRust<LaserPreview>('laser_frame_preview',plan)

export const preflightLaserPlan=(plan:LaserPlan):LaserSummary=>
  previewLaserPlan(plan).summary

export const emitLaserGrbl=(plan:LaserPlan):LaserProgram=>
  callGeometryRust<LaserProgram>('laser_grbl',plan)

export const emitLaserFrame=(plan:LaserPlan):LaserProgram=>
  callGeometryRust<LaserProgram>('laser_frame',plan)
