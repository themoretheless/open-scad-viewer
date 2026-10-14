import {expect,it} from 'vitest'
import {callNurbsRust} from '../src/services/geometry/nurbs'
import {createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import type {NurbsCurve} from '../src/services/nurbsCurve'
it('transports complete retained wall domains and exact coedges under one work budget',()=>{
 const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
 const ring=(z:number)=>[[line([0,0,z],[1,0,z]),line([1,0,z],[1,1,z]),line([1,1,z],[0,1,z]),line([0,1,z],[0,0,z])]]
 const model=createRationalBrepSectionLoft([ring(0),ring(10)]),face=model.faces[0]!
 const request={surface:face.surface,holes:face.holes,coedges:model.loops[face.outer]!.coedges.map(use=>({world:model.edges[use.edge]!.curve,uv:use.pcurve,reversed:use.reversed}))}
 const before=structuredClone(request)
 const audit=(value=request,maxWork=1000000)=>callNurbsRust<{domainCertified:boolean;work:number;globalEmbeddingCertified:false}>('sweep_retained_wall_domain_audit',{...value,maxWork})
 const result=audit();expect(result.domainCertified).toBe(true);expect(result.globalEmbeddingCertified).toBe(false)
 expect(result.work).toBeGreaterThan(0)
 expect(audit(request,result.work).domainCertified).toBe(true)
 expect(audit(request,result.work-1).domainCertified).toBe(false)
 const rotated=structuredClone(request);rotated.coedges.push(rotated.coedges.shift()!)
 expect(audit(rotated).domainCertified).toBe(true)
 rotated.coedges.reverse();for(const use of rotated.coedges){use.uv.controlPoints.reverse();use.reversed=!use.reversed}
 expect(audit(rotated).domainCertified).toBe(true)
 const rational=structuredClone(request)
 for(const use of rational.coedges){use.uv.weights=[2,1];use.world.weights=use.reversed?[1,2]:[2,1]}
 expect(audit(rational).domainCertified).toBe(true)
 rational.coedges[0]!.world.weights=[1,1]
 expect(audit(rational).domainCertified).toBe(false)
 const edge=structuredClone(request);edge.coedges[0]!.world.controlPoints[0]![2]!+=.125
 expect(audit(edge).domainCertified).toBe(false)
 const clipped=structuredClone(request);clipped.coedges[0]!.uv.controlPoints[1]![0]=.5
 expect(audit(clipped).domainCertified).toBe(false)
 const hole=structuredClone(request);hole.holes=[0]
 expect(audit(hole).domainCertified).toBe(false)
 expect(()=>audit(request,1000001)).toThrow()
 expect(request).toEqual(before)
})
