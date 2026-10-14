import {expect,it} from 'vitest'
import {circleNurbsCurve,miterNurbsProfileSections} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {projectNurbsSection,projectSweepSections,projectAuthoredNurbsSection} from '../src/services/nurbsSectionProjection'
import {createRationalBrepSectionLoft,createMiterBrepProfileBody} from '../src/services/geometry/brep'
import {inspectSweepEmbedding,inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

it('corrects the actual spatial endpoint once for walls and cap with a bounded curve displacement',()=>{
 const profiles=[circleNurbsCurve([0,0,0],[0,0,1],.5),reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]
 const sections=miterNurbsProfileSections(profiles,[[0,0,0],[0,0,10],[10,0,10],[10,10,15]],[1,0,0],2)
 const before=structuredClone(sections),endpoint=sections.at(-1)!
 const plane={axis:1 as const,coefficients:[0,-.5] as [number,number],offset:17.5}
 const options={quantum:2**-40,tolerance:1e-9,maxWork:1000000}
 const correction=projectNurbsSection(endpoint,plane,options)
 expect(correction).toMatchObject({exactPlanar:true,reason:'bounded-exact-plane'})
 expect(correction.displacementUpper).toBeLessThanOrEqual(options.tolerance)
 expect(correction.curves?.map(c=>c.weights)).toEqual(endpoint.map(c=>c.weights))
 expect(sections).toEqual(before)
 expect(projectNurbsSection(endpoint,plane,{...options,maxWork:0})).toMatchObject({curves:null,exactPlanar:false})
 expect(projectNurbsSection(endpoint,plane,{...options,tolerance:0})).toMatchObject({curves:null,exactPlanar:false})
 const corrected=projectSweepSections(sections,[{section:sections.length-1,plane,quantum:options.quantum,tolerance:options.tolerance}],options.maxWork)
 expect(corrected.reason).toBe('bounded-section-interpolation')
 expect(corrected.wallDisplacementUpper).toBe(correction.displacementUpper)
 expect(corrected.sections?.at(-1)).toEqual(correction.curves)
 expect(sections).toEqual(before)
 const startPlane={axis:2 as const,coefficients:[0,0] as [number,number],offset:0}
 const start=projectNurbsSection(sections[0]!,startPlane,options)
 expect(start.exactPlanar).toBe(true)
 const shared=projectSweepSections(sections,[
  {section:0,plane:startPlane,quantum:options.quantum,tolerance:options.tolerance},
  {section:sections.length-1,plane,quantum:options.quantum,tolerance:options.tolerance},
 ],start.work)
 expect(shared).toMatchObject({sections:null,wallDisplacementUpper:null,reason:'work-limit',work:start.work})
 expect(projectSweepSections(sections,[{section:sections.length-1,plane,quantum:options.quantum,tolerance:options.tolerance}],0).wallDisplacementUpper).toBeNull()
 const changed=structuredClone(sections);changed[1]![0]!.weights[0]=2
 expect(projectSweepSections(changed,[{section:sections.length-1,plane,quantum:options.quantum,tolerance:options.tolerance}],options.maxWork).reason).toBe('incompatible-section-basis')
 sections.splice(0,sections.length,...corrected.sections!)
 const model=createRationalBrepSectionLoft(sections.map(row=>row.map(c=>[c])))
 const report=inspectSweepEmbedding(model,[model.faces.length-2,model.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(report.exactBoundaryCertified).toBe(true)
 expect(report.caps.every(c=>c.capCertified && c.unresolvedWalls.length===0)).toBe(true)
 expect(report.boundaryEmbeddingCertified).toBe(true)
 const volume=inspectSweepVolume(model,[model.faces.length-2,model.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS)
 expect(volume.solidGeometryCertified).toBe(true)
 expect(report.allFacesInjective).toBe(true)
 expect(report.unresolvedFaces).toEqual([])
 expect(report.linearCells).toBeLessThanOrEqual(DEFAULT_SWEEP_VOLUME_BUDGETS.maxLinearCells)
})

it('constructs both corrected caps and walls through the ordinary miter body entrypoint',()=>{
 const outer=circleNurbsCurve([0,0,0],[0,0,1],.5),hole=reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))
 const loops=[[outer],[hole]],points:[number,number,number][]=[[0,0,0],[0,0,10],[10,0,10],[10,10,15]]
 const before=structuredClone(loops)
 const options={quantum:2**-40,tolerance:1e-9,maxWork:1000000}
 const body=createMiterBrepProfileBody(loops,points,[1,0,0],2,false,options)
 expect(body.report.sectionCorrection?.exactPlanarSections).toEqual([0,3])
 expect(body.report.sectionCorrection?.wallDisplacementUpper).toBeLessThan(options.tolerance)
 expect(body.report.retainedCorrespondence?.exact).toBe(true)
 expect(body.report.wallRegularityCertified).toBe(true)
 expect(body.report.retainedWallCharts.charts).toHaveLength(24)
 expect(body.report.retainedWallCharts.unresolvedFaces).toEqual([])
 expect(body.report.retainedWallCharts.cells).toBeLessThanOrEqual(DEFAULT_SWEEP_VOLUME_BUDGETS.maxLinearCells)
 expect(body.report.continuousBound).toBe(false)
 expect(inspectSweepVolume(body.model,[24,25],DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(true)
 expect(loops).toEqual(before)
 expect(()=>createMiterBrepProfileBody(loops,points,[1,0,0],2,false,{...options,maxWork:0})).toThrow(/correction unproved/)
 expect(()=>createMiterBrepProfileBody(loops,points,[1,0,0],2,false,{...options,tolerance:0})).toThrow(/correction unproved/)
})

it('retains periodic rational bases during bounded section correction',()=>{
 const c={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,.01],[0,1,.01],[-1,0,.01],[0,-1,.01],[1,0,.01],[0,1,.01]],weights:Array(6).fill(1),periodic:true}
 const sections=[[c],[structuredClone(c)]],before=structuredClone(sections)
 const corrections=[{section:1,plane:{axis:2 as const,coefficients:[0,0] as [number,number],offset:0},quantum:2**-40,tolerance:.02}]
 const corrected=projectSweepSections(sections,corrections,1000000)
 expect(corrected).toMatchObject({reason:'bounded-section-interpolation',exactPlanarSections:[1]})
 expect(corrected.wallDisplacementUpper).toBeLessThan(.02)
 expect(corrected.sections![1]![0]).toMatchObject({periodic:true,knots:c.knots,weights:c.weights})
 expect(corrected.sections![1]![0]!.controlPoints.every(p=>p[2]===0)).toBe(true)
 expect(sections).toEqual(before)
 const incompatible=structuredClone(sections);incompatible[1]![0]!.periodic=false
 expect(projectSweepSections(incompatible,corrections,1000000).reason).toBe('incompatible-section-basis')
})

it('projects a periodic cap along its moving authored axis instead of the path axis',()=>{
 const z=10+.25*Math.sqrt(1.25)
 const xy=[[1,0],[0,1],[-1,0],[0,-1],[1,0],[0,1]]
 const c={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:xy.map(([x,y])=>[x!,y!,z-.5*y!]),weights:Array(6).fill(1),periodic:true}
 const axis={degree:1,knots:[2,2,5,5],controlPoints:[[0,0,1],[0,.5,1]],weights:[1,1],periodic:false}
 const before=structuredClone({c,axis}),opts={quantum:2**-40,tolerance:1e-9,maxWork:1000000}
 const projected=projectAuthoredNurbsSection([c],axis,1,opts)
 expect(projected).toMatchObject({exactPlanar:true,reason:'bounded-exact-plane'})
 expect(projected.displacementUpper).toBeLessThan(opts.tolerance)
 expect(projected.curves![0]).toMatchObject({periodic:true,knots:c.knots,weights:c.weights})
 expect(projectNurbsSection([c],{axis:2,coefficients:[0,0],offset:10},opts)).toMatchObject({curves:null,reason:'displacement-budget'})
 expect(projectAuthoredNurbsSection([c],axis,1,{...opts,maxWork:0})).toMatchObject({curves:null,exactPlanar:false})
 const zero={...structuredClone(axis),controlPoints:[[0,0,0],[0,0,0]]}
 expect(()=>projectAuthoredNurbsSection([c],zero,1,opts)).toThrow('Authored section axis must be finite and nonzero')
 expect({c,axis}).toEqual(before)
})
