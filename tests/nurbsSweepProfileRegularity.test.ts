import {expect,it} from 'vitest'
import {inspectSweepProfileRegularity} from '../src/services/nurbsSweepAudit'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'

it('covers outer and hole tangents continuously with one shared budget',()=>{
 const profiles=[circleNurbsCurve([0,0,0],[0,0,1],.5),circleNurbsCurve([0,0,0],[0,0,-1],.2)]
 const before=structuredClone(profiles)
 const report=inspectSweepProfileRegularity(profiles,10000)
 expect(report).toMatchObject({spanwiseRegular:true,unresolvedProfiles:[],continuityCertified:false})
 expect(report.cells).toBeGreaterThan(0)
 expect(report.cells).toBeLessThanOrEqual(10000)
 const first=inspectSweepProfileRegularity([profiles[0]!],10000)
 const short=inspectSweepProfileRegularity(profiles,first.cells)
 expect(short.spanwiseRegular).toBe(false)
 expect(short.unresolvedProfiles).toContain(1)
 expect(short.cells).toBeLessThanOrEqual(first.cells)
 expect(inspectSweepProfileRegularity(profiles,0)).toMatchObject({spanwiseRegular:false,cells:0,unresolvedProfiles:[0,1]})
 expect(profiles).toEqual(before)
})

it('refuses stationary tangents and never claims corner continuity',()=>{
 const stationary={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0,0],[1,0,0],[0,0,0]],weights:[1,1,1],periodic:false}
 expect(inspectSweepProfileRegularity([stationary],63)).toMatchObject({spanwiseRegular:false,unresolvedProfiles:[0],continuityCertified:false})
 const corner={degree:1,knots:[0,0,.5,1,1],controlPoints:[[0,0,0],[1,0,0],[1,1,0]],weights:[1,1,1],periodic:false}
 expect(inspectSweepProfileRegularity([corner],2)).toMatchObject({spanwiseRegular:true,cells:2,continuityCertified:false})
 expect(()=>inspectSweepProfileRegularity([],2)).toThrow(/Invalid profile regularity budget/)
 expect(()=>inspectSweepProfileRegularity([corner],100001)).toThrow(/Invalid profile regularity budget/)
 expect(()=>inspectSweepProfileRegularity(Array.from({length:65},()=>corner),2)).toThrow(/Invalid profile regularity budget/)
})
