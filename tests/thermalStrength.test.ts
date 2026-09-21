import {expect,it} from 'vitest'
import {evaluateThermalStrength,screenTrussMembers,type ThermalStrengthRequest} from '../src/services/trussScreening'
import {solveTruss,type TrussModel} from '../src/services/trussAnalysis'
import {callGeometryRust} from '../src/services/geometry/kernel'

const profile={material:'PETG',grade:'Synthetic specimen',propertySource:'Regression only',nozzleMm:0.4,lineWidthMm:0.45,layerHeightMm:0.2,nozzleTempC:240,bedTempC:80}
const request:ThermalStrengthRequest={profile,calibrationProfile:{...profile},serviceTempC:20,samples:[
  {nozzleTempC:240,serviceTempC:20,youngMpa:2000,tensionMpa:40,compressionMpa:60},
  {nozzleTempC:240,serviceTempC:60,youngMpa:1000,tensionMpa:20,compressionMpa:30},
]}

it('changes real WASM displacement and utilization from the temperature curve',()=>{
  const evaluate=(serviceTempC:number)=>{
    const thermal=evaluateThermalStrength({...request,serviceTempC})
    const model:TrussModel={nodesMm:[[0,0,0],[10,0,0]],members:[{nodes:[0,1],areaMm2:2,youngMpa:thermal.youngMpa}],
      restrained:[[true,true,true],[false,true,true]],forcesN:[[0,0,0],[100,0,0]]}
    const response=solveTruss(model)
    return {thermal,response,screening:screenTrussMembers(model,[response],{tensionMpa:thermal.tensionMpa,compressionMpa:thermal.compressionMpa,safetyFactor:2})}
  }
  const cold=evaluate(20),hot=evaluate(60),mid=evaluate(40)
  expect(cold.response.maxDeflectionMm).toBeCloseTo(0.25,12)
  expect(hot.response.maxDeflectionMm).toBeCloseTo(0.5,12)
  expect(mid.thermal.youngMpa).toBe(1500)
  expect(cold.screening[0].utilization).toBeCloseTo(2.5,12)
  expect(hot.screening[0].utilization).toBeCloseTo(5,12)
  expect(mid.thermal.serviceBracketC).toEqual([20,60])
})

it('interpolates both axes without assuming increasing nozzle temperature improves strength',()=>{
  const grid={...request,profile:{...profile,nozzleTempC:250},serviceTempC:40,samples:[...request.samples,
    {nozzleTempC:260,serviceTempC:20,youngMpa:1000,tensionMpa:20,compressionMpa:30},
    {nozzleTempC:260,serviceTempC:60,youngMpa:500,tensionMpa:10,compressionMpa:15}]}
  expect(evaluateThermalStrength(grid)).toMatchObject({youngMpa:1125,tensionMpa:22.5,compressionMpa:33.75,nozzleBracketC:[240,260]})
  expect(evaluateThermalStrength({...grid,samples:grid.samples.toReversed()})).toEqual(evaluateThermalStrength(grid))
})

it('refuses extrapolation, missing cells, duplicate measurements and mismatched print processes',()=>{
  for(const serviceTempC of [19,61])expect(()=>evaluateThermalStrength({...request,serviceTempC})).toThrowError(expect.objectContaining({code:'THERMAL_OUT_OF_RANGE'}))
  expect(()=>evaluateThermalStrength({...request,profile:{...profile,nozzleTempC:241}})).toThrowError(expect.objectContaining({code:'THERMAL_OUT_OF_RANGE'}))
  for(const change of [{grade:'Other'},{nozzleMm:0.6},{lineWidthMm:0.5},{layerHeightMm:0.3},{bedTempC:90},{material:'PLA'},{propertySource:'Other test'}]) {
    expect(()=>evaluateThermalStrength({...request,profile:{...profile,...change}})).toThrowError(expect.objectContaining({code:'THERMAL_PROFILE_MISMATCH'}))
  }
  expect(()=>evaluateThermalStrength({...request,samples:[request.samples[0],request.samples[0]]})).toThrow()
  expect(()=>evaluateThermalStrength({...request,samples:[...request.samples,{...request.samples[0],nozzleTempC:260}]})).toThrow()
  expect(()=>evaluateThermalStrength({...request,samples:Array(65).fill(request.samples[0])})).toThrow()
  expect(()=>callGeometryRust('thermal_strength',{...request,glassTransitionC:80})).toThrow()
  expect(()=>evaluateThermalStrength({...request,samples:[{...request.samples[0],youngMpa:NaN},request.samples[1]]})).toThrow()
})
