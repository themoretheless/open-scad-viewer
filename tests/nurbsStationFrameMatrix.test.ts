import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveBrepProfileBody,reconstructCertifiedProgressiveStations,type ProgressiveBrepSweepOptions} from '../src/services/geometry/brep'

// Explicit finite companions for every declared frame/spacing choice.
// Passing this matrix does not certify arbitrary source curves or law tuples.
const modes=['rmf','fixed','fixed_normal','frenet','corrected_frenet','authored'] as const
const spacings=['parameter','arc_length'] as const
const cases=modes.flatMap(orientation=>spacings.map(spacing=>({orientation,spacing})))
const scalar=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const vector=(value:[number,number,number])=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
it.each(cases)('retains complete bound, actual G2 and Solid for $orientation / $spacing',({orientation,spacing})=>{
 const loops=[[circleNurbsCurve([0,0,0],[1,0,0],.05)],
  [reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.025))]]
 const path=bezierNurbsCurve([[0,0,0],[.5,0,0],[1,1,0]])
 const correction={quantum:2**-40,tolerance:1e-9,maxWork:1000000}
 const options:ProgressiveBrepSweepOptions={orientation,spacing,normal:[0,0,1],initialSections:17,maxSections:17,
  maxDeviation:.05,lengthTolerance:.0001,lengthMaxCells:100000,circleCorrection:correction,capCorrection:correction,
  ...(orientation==='authored'?{frameAxis:vector([1,0,0]),frameNormal:vector([0,0,1])}:{})}
 const original=JSON.stringify({loops,path,options})
 const source=createProgressiveBrepProfileBody(loops,path,scalar(1),scalar(0),options)
 expect(source.boundaryContinuousBound).toBe(true)
 const saved=JSON.stringify(source.model)
 const retained=reconstructCertifiedProgressiveStations(source,{quantum:2**-40,wallTolerance:.01,maxWork:1000000,smoothnessMaxWork:2000000})
 expect(retained.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true,budget:.05})
 expect(retained.profileSmoothness).toMatchObject({profile:{exactG1G2Certified:true},station:{stationG2Certified:true}})
 expect(retained.volume).toMatchObject({solidGeometryCertified:true,allFacesInjective:true,allPairsClassified:true})
 expect(JSON.stringify(source.model)).toBe(saved)
 expect(JSON.stringify({loops,path,options})).toBe(original)
})
