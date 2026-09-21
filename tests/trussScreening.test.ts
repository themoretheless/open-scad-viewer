import {expect, it} from 'vitest'
import {screenTrussMembers,validatePrintStrengthProfile} from '../src/services/trussScreening'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {solveTruss, type TrussModel, type TrussResponse} from '../src/services/trussAnalysis'

const model:TrussModel={nodesMm:[[0,0,0],[10,0,0]],members:[{nodes:[0,1],youngMpa:2000,areaMm2:2}],
  restrained:[[true,true,true],[false,true,true]],forcesN:[[0,0,0],[100,0,0]]}
const limits={tensionMpa:80,compressionMpa:40,safetyFactor:2}

it('screens a real native axial solution using separate tensile and compressive limits',()=>{
  const tension=solveTruss(model)
  expect(screenTrussMembers(model,[tension],limits)[0]).toMatchObject({utilization:1.25,status:'overloaded',stressMpa:50})
  const compression=solveTruss({...model,forcesN:[[0,0,0],[-60,0,0]]})
  const before=structuredClone(model)
  expect(screenTrussMembers(model,[tension,compression],limits)[0]).toMatchObject({utilization:1.5,governingResponse:1,stressMpa:-30})
  expect(model).toEqual(before)
})

it('keeps exactly full utilization within limit and zero demand as a screening candidate',()=>{
  const response=solveTruss(model)
  const evaluate=(force:number)=>screenTrussMembers(model,[{...response,axialForcesN:[force]}],limits)[0]
  expect(evaluate(80).status).toBe('within-axial-limit')
  expect(evaluate(24).status).toBe('within-axial-limit')
  expect(evaluate(0)).toMatchObject({utilization:0,status:'low-demand'})
})

it('refuses incomplete or nonfinite results, invalid limits and numeric overflow',()=>{
  const response=solveTruss(model)
  for(const value of [0,-1,NaN,Infinity]) {
    expect(()=>screenTrussMembers(model,[response],{...limits,tensionMpa:value})).toThrow()
    expect(()=>screenTrussMembers(model,[response],{...limits,compressionMpa:value})).toThrow()
  }
  expect(()=>screenTrussMembers(model,[response],{...limits,safetyFactor:0.5})).toThrow()
  expect(()=>screenTrussMembers(model,[],limits)).toThrow()
  for(const forces of [[],[NaN],new Array(1)])expect(()=>screenTrussMembers(model,[{...response,axialForcesN:forces}],limits)).toThrow()
  expect(()=>screenTrussMembers({...model,members:[{...model.members[0],areaMm2:Number.MIN_VALUE}]},[response],limits)).toThrow()
})

it('ranks the worst utilization first and retains original member indices',()=>{
  const response={axialForcesN:[0,80,-60]} as TrussResponse
  const expanded={...model,members:Array.from({length:3},()=>({...model.members[0]}))}
  expect(screenTrussMembers(expanded,[response],limits).map(row=>row.memberIndex)).toEqual([2,1,0])
})

it('validates a print profile in WASM and preserves exact settings without inferred properties',()=>{
  const profile={material:'PLA',grade:'Test grade',propertySource:'Measured coupon',nozzleMm:0.4,lineWidthMm:0.45,layerHeightMm:0.2,nozzleTempC:210,bedTempC:60}
  expect(validatePrintStrengthProfile(profile)).toEqual({profile,warnings:[],propertyModel:'user-supplied-isotropic-axial'})
  expect(validatePrintStrengthProfile({...profile,layerHeightMm:0.4}).warnings).toEqual(['layer-above-80-percent-nozzle'])
  for(const invalid of [{material:'unknown'},{propertySource:''},{nozzleMm:0},{nozzleTempC:0},{bedTempC:-1}]) {
    expect(()=>validatePrintStrengthProfile({...profile,...invalid})).toThrowError(expect.objectContaining({code:'PRINT_PROFILE_INVALID_INPUT'}))
  }
  expect(()=>callGeometryRust('print_strength_profile',{profile:{...profile,temperatureFactor:1.2}})).toThrow()
})
