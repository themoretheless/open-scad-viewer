import type {LaserMachineProfile} from './geometry/laser'

export interface LaserMachinePreset {
  id:string
  widthMm:number
  heightMm:number
  maxPower:number
  estimatedRapidMmMin:number
  powerMode:'m3'|'m4'
  supportsAirAssist:boolean
}

export interface LaserMaterialPreset {
  id:string
  line:{speedMmMin:number;powerFraction:number;passes:number;airAssist:boolean;kerfMm:number}
  fill:{speedMmMin:number;powerFraction:number;passes:number;airAssist:boolean;spacingMm:number}
}

export const LASER_MACHINE_PRESETS:readonly LaserMachinePreset[]=[
  {id:'desktop-300x200',widthMm:300,heightMm:200,maxPower:1000,estimatedRapidMmMin:5000,powerMode:'m4',supportsAirAssist:false},
  {id:'generic-400x400',widthMm:400,heightMm:400,maxPower:1000,estimatedRapidMmMin:6000,powerMode:'m4',supportsAirAssist:false},
  {id:'large-600x400',widthMm:600,heightMm:400,maxPower:1000,estimatedRapidMmMin:8000,powerMode:'m4',supportsAirAssist:true},
]

// Conservative starting points, never qualified recipes. The panel keeps a
// calibration warning visible after applying any material preset.
export const LASER_MATERIAL_PRESETS:readonly LaserMaterialPreset[]=[
  {id:'cardboard',line:{speedMmMin:1200,powerFraction:0.3,passes:1,airAssist:false,kerfMm:0.15},fill:{speedMmMin:2400,powerFraction:0.18,passes:1,airAssist:false,spacingMm:0.2}},
  {id:'plywood-3mm',line:{speedMmMin:600,powerFraction:0.8,passes:2,airAssist:true,kerfMm:0.2},fill:{speedMmMin:1800,powerFraction:0.22,passes:1,airAssist:false,spacingMm:0.18}},
  {id:'acrylic-3mm',line:{speedMmMin:420,powerFraction:0.9,passes:2,airAssist:true,kerfMm:0.18},fill:{speedMmMin:1500,powerFraction:0.2,passes:1,airAssist:false,spacingMm:0.16}},
]

export function machineProfileFromPreset(
  preset:LaserMachinePreset,
  previous:LaserMachineProfile,
):LaserMachineProfile {
  return {
    ...previous,
    widthMm:preset.widthMm,
    heightMm:preset.heightMm,
    maxPower:preset.maxPower,
    estimatedRapidMmMin:preset.estimatedRapidMmMin,
    powerMode:preset.powerMode,
    supportsAirAssist:preset.supportsAirAssist,
    // A preset cannot attest to the connected controller state.
    laserModeConfirmed:false,
  }
}

export const presetPower=(fraction:number,maxPower:number)=>
  Math.max(1,Math.min(maxPower,Math.round(fraction*maxPower)))
