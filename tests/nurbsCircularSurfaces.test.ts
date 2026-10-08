import {expect,it} from 'vitest'
import {sphereNurbsSurface,cylinderNurbsSurface,coneNurbsSurface} from '../src/services/nurbsConstructors'
import {evaluateNurbsSurface,type NurbsSurface} from '../src/services/nurbsSurface'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {readFileSync} from 'node:fs'

it('preserves independent sphere, cylinder and cone equations through the packaged kernel',()=>{
  const center:[number,number,number]=[1,-2,3]
  const surfaces:NurbsSurface[]=[sphereNurbsSurface(center,5),cylinderNurbsSurface(center,5,7),coneNurbsSurface(center,5,7)]
  surfaces.forEach((s,kind)=>{
    const u0=s.knotsU[s.degreeU]!,u1=s.knotsU[s.controlPoints.length]!
    const v0=s.knotsV[s.degreeV]!,v1=s.knotsV[s.controlPoints[0]!.length]!
    for(const u of [0,.17,.5,.93,1])for(const v of [0,.13,.5,.89,1]){
      const p=evaluateNurbsSurface(s,u0+(u1-u0)*u,v0+(v1-v0)*v).point
      const radial=Math.hypot(p[0]!-center[0],p[1]!-center[1]),z=p[2]!-center[2]
      const error=kind===0?radial*radial+z*z-25:kind===1?radial-5:radial-5*(1-z/7)
      expect(Math.abs(error)).toBeLessThan(1e-11)
      if(kind>0){expect(z).toBeGreaterThanOrEqual(-1e-12);expect(z).toBeLessThanOrEqual(7+1e-12)}
    }
  })
  for(const radius of [0,-1,NaN,Infinity]){
    expect(()=>sphereNurbsSurface(center,radius)).toThrow()
    expect(()=>cylinderNurbsSurface(center,radius,7)).toThrow()
    expect(()=>coneNurbsSurface(center,radius,7)).toThrow()
  }
})

it('requires lengths for circular surface dimensions in Rush',()=>{
  for(const name of ['sphere','cylinder','cone']){
    const source=readFileSync(`examples/rush/${name}-surface.r`,'utf8')
    const graph=compileRushFrontend(source)
    expect(graph.execution_target).toBe('own-nurbs')
    expect(graph.document.nodes.some(n=>n.op===`${name}_surface`)).toBe(true)
    expect(()=>compileRushFrontend(source.replace(/radius: \d+mm/,'radius: 8deg'))).toThrow()
  }
})
