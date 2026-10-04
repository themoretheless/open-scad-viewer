import {callGeometryRust} from './kernel'

export type LaserPowerMode='m3'|'m4'
export type LaserOperationKind='line'|'fill'

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

export const preflightLaserPlan=(plan:LaserPlan):LaserSummary=>
  callGeometryRust<{summary:LaserSummary}>('laser_preflight',plan).summary

export const emitLaserGrbl=(plan:LaserPlan):LaserProgram=>
  callGeometryRust<LaserProgram>('laser_grbl',plan)

export const emitLaserFrame=(plan:LaserPlan):LaserProgram=>
  callGeometryRust<LaserProgram>('laser_frame',plan)
