import {describe,expect,it} from 'vitest'
import {booleanBrepProfiles,signedAreaBrepProfileLoop,validateBrepProfile} from '../src/services/geometry/brepProfile'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {evaluateNurbsCurve,reverseNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'

function rectangle(x:number,y:number,w:number,h:number):NurbsCurve[] {
  const p=[[x,y],[x+w,y],[x+w,y+h],[x,y+h]]
  return p.map((point,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[point,p[(i+1)%4]],weights:[1,1]}))
}
function circle(cx:number,cy:number,r:number):NurbsCurve[] {
  const axes=[[1,0],[0,1],[-1,0],[0,-1]]
  return axes.map((a,i)=>{
    const b=axes[(i+1)%4]
    return {degree:2,knots:[0,0,0,1,1,1],controlPoints:[[cx+r*a[0],cy+r*a[1]],[cx+r*(a[0]+b[0]),cy+r*(a[1]+b[1])],[cx+r*b[0],cy+r*b[1]]],weights:[1,Math.SQRT1_2,1]}
  })
}
const reverse=(wire:NurbsCurve[])=>wire.toReversed().map(reverseNurbsCurve)

describe('retained 2D B-rep profiles',()=>{
  it('orients unordered even-odd rings, preserving holes and nested islands',()=>{
    const rings=[rectangle(2,2,6,6),reverse(rectangle(0,0,10,10)),reverse(rectangle(4,4,2,2))]
    const before=JSON.stringify(rings)
    const profile=validateBrepProfile(rings,'even-odd')
    expect(profile).toMatchObject({kind:'brep-profile',areaMm2:68,toleranceMm:1e-7,geometryStatus:'numerical_uncertified'})
    expect(profile.loops.map(wire=>signedAreaBrepProfileLoop(wire))).toEqual([-36,100,4])
    expect(JSON.stringify(rings)).toBe(before)
    expect(()=>validateBrepProfile(rings)).toThrow(/orientation/)
    expect(profile).not.toHaveProperty('mesh')
    expect(profile).not.toHaveProperty('vertices')
    expect(validateBrepProfile(JSON.parse(JSON.stringify(profile.loops)))).toEqual(profile)
  })

  it('uses circle/line analytic Boolean with retained rational arcs',()=>{
    const a=validateBrepProfile([circle(0,0,2)])
    const b=validateBrepProfile([rectangle(0,-3,3,6)])
    expect(a.areaMm2).toBeCloseTo(4*Math.PI,11)
    const expected={intersection:2*Math.PI,difference:2*Math.PI,union:18+2*Math.PI,xor:18}
    for(const operation of ['intersection','difference','union','xor'] as const){
      const result=booleanBrepProfiles(a,b,operation)
      expect(result.areaMm2).toBeCloseTo(expected[operation],10)
      const curves=result.loops.flat().filter(c=>c.degree===2)
      expect(curves.length).toBeGreaterThan(0)
      for(const curve of curves) for(const fraction of [0,.19,.5,.81,1]){
        const lo=curve.knots[curve.degree],hi=curve.knots[curve.controlPoints.length]
        const p=evaluateNurbsCurve(curve,lo+fraction*(hi-lo)).point
        expect(p[0]**2+p[1]**2).toBeCloseTo(4,10)
      }
    }
    const annulus=booleanBrepProfiles(a,validateBrepProfile([circle(0,0,1)]),'difference')
    expect(annulus.loops).toHaveLength(2)
    expect(annulus.areaMm2).toBeCloseTo(3*Math.PI,10)
    expect(annulus.loops.map(wire=>Math.sign(signedAreaBrepProfileLoop(wire))).sort()).toEqual([-1,1])
  })

  it('returns canonical empty and keeps source inputs unchanged',()=>{
    const a=validateBrepProfile([circle(2,3,4)])
    const before=JSON.stringify(a)
    const empty=validateBrepProfile([],'even-odd')
    expect(empty).toEqual({kind:'brep-profile',loops:[],areaMm2:0,toleranceMm:1e-7,geometryStatus:'numerical_uncertified'})
    expect(booleanBrepProfiles(a,a,'difference')).toEqual(empty)
    expect(booleanBrepProfiles(a,a,'xor')).toEqual(empty)
    expect(booleanBrepProfiles(a,empty,'intersection')).toEqual(empty)
    expect(booleanBrepProfiles(a,empty,'union').areaMm2).toBeCloseTo(a.areaMm2,10)
    expect(JSON.stringify(a)).toBe(before)
    expect(()=>signedAreaBrepProfileLoop([])).toThrow(/nonempty/)
  })

  it('refuses crossings, contacting holes, near coincidence and existing resource overflow',()=>{
    expect(()=>validateBrepProfile([rectangle(0,0,2,2),rectangle(1,1,2,2)],'even-odd')).toThrow()
    expect(()=>validateBrepProfile([rectangle(0,0,4,4),rectangle(0,1,2,2)],'even-odd')).toThrow()
    const a=validateBrepProfile([rectangle(10000,0,1,1)])
    const b=validateBrepProfile([rectangle(10000,1e-10,1,1)])
    expect(()=>booleanBrepProfiles(a,b,'union')).toThrow(GeometryKernelError)
    expect(()=>validateBrepProfile(Array.from({length:257},(_,i)=>rectangle(i*3,0,1,1)))).toThrow(/256|resource/i)
    expect(()=>validateBrepProfile([], 'material-left',0)).toThrow(/tolerance/i)
  })
})
