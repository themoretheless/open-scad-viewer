import {expect,it} from 'vitest'
import {solveBondedSolid} from '../src/services/bondedSolid'
import {parseBondedSolidInput,isBondedSolidResult} from '../src/services/bondedSolidProtocol'
import {mainSolidExpectation,mainSolidResult} from '../src/services/mainSolidProtocol'
import {bondedSolidExample} from '../src/features/bondedSolidExample'
const input=()=>structuredClone(bondedSolidExample)
it('solves explicit Tet4 bonds through WASM with area-derived traction and balanced reactions',()=>{
  const m=input(),text=JSON.stringify(m),r=solveBondedSolid(text)
  expect(r.geometryBinding).toBe('explicit-mesh-not-cad-verified')
  expect(r.bonds[0].areaMm2).toBeCloseTo(.5,12)
  expect(r.bonds[0].forceOnShellN[2]).toBeCloseTo(3,10)
  expect(r.bonds[0].utilization).toBeCloseTo(.6,10)
  expect(r.displacementsMm[7][2]).toBeCloseTo(.06,10)
  expect(r.reactionsN.reduce((a,b)=>a+b[2],0)).toBeCloseTo(-3,10)
  expect(r.limitReached).toBe(false)
  const expected=mainSolidExpectation({kind:'bondedSolid',inputJson:text})
  expect(mainSolidResult(expected,r)).toBe(true)
  expect(mainSolidResult(expected,{...r,limitReached:true})).toBe(false)
  expect(isBondedSolidResult({...r,bonds:[]},8,2,1)).toBe(false)
  expect(isBondedSolidResult({...r,stressesMpa:[[NaN]]},8,2,1)).toBe(false)
})
it('screens combined tension and shear and labels exceeded design limits without degrading the model',()=>{
  const m=input();m.forcesN[4]=[1,0,1];m.forcesN[5]=[1,0,1];m.forcesN[6]=[1,0,1];m.safetyFactor=2
  const r=solveBondedSolid(JSON.stringify(m))
  expect(r.bonds[0].utilization).toBeCloseTo(2*Math.hypot(.6,.75),10)
  expect(r.limitReached).toBe(true)
  expect(r.bonds[0].forceOnShellN[0]).toBeCloseTo(3,10)
})
it('rejects missing calibration, bypasses, unknown fields and singular assemblies',()=>{
  for(const change of [
    (m:ReturnType<typeof input>)=>{m.bonds[0].propertySource=''},
    (m:ReturnType<typeof input>)=>{m.tets[1].nodes[0]=0},
    (m:ReturnType<typeof input>)=>{m.restrained.fill([false,false,false])},
    (m:ReturnType<typeof input>)=>{m.bonds[0].shearStiffnessMpaPerMm=0},
    (m:ReturnType<typeof input>)=>{m.profile.nozzleMm=0},
    (m:ReturnType<typeof input>)=>{m.serviceTempC=-300},
  ]){const m=input();change(m);expect(()=>solveBondedSolid(JSON.stringify(m))).toThrow()}
  expect(()=>solveBondedSolid(JSON.stringify({...input(),automaticSupports:true}))).toThrow()
})
it('bounds the request before crossing the worker boundary',()=>{
  expect(()=>parseBondedSolidInput(' '.repeat(262145))).toThrow()
  expect(()=>parseBondedSolidInput(JSON.stringify({...input(),tets:Array(401).fill(input().tets[0])}))).toThrow()
  expect(()=>parseBondedSolidInput('null')).toThrow()
})
