import {describe,expect,it} from 'vitest'
import {LASER_MACHINE_PRESETS,LASER_MATERIAL_PRESETS,machineProfileFromPreset,presetPower} from '../src/services/laserProfiles'

describe('laser CAM profiles',()=>{
  it('never carries live laser-mode confirmation across a machine preset',()=>{
    const profile=machineProfileFromPreset(LASER_MACHINE_PRESETS[0]!,{
      widthMm:1,heightMm:1,maxPower:500,estimatedRapidMmMin:100,
      powerMode:'m3',laserModeConfirmed:true,supportsAirAssist:true,
      flipY:true,returnToOrigin:false,
    })
    expect(profile).toMatchObject({widthMm:300,heightMm:200,laserModeConfirmed:false,flipY:true,returnToOrigin:false})
  })

  it('keeps preset power inside the controller range',()=>{
    expect(presetPower(0.8,1000)).toBe(800)
    expect(presetPower(2,1000)).toBe(1000)
    expect(presetPower(0,1000)).toBe(1)
    expect(LASER_MATERIAL_PRESETS.every(preset=>preset.line.kerfMm>0)).toBe(true)
  })
})
