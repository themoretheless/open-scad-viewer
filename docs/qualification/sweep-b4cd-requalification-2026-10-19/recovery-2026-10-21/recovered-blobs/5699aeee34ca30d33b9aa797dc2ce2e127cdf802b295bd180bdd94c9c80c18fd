import type {DirectSketch} from './directModeling'
import type {MainSolidJob} from './mainSolidProtocol'
import type {SnapGeometry} from './modelingSnaps'
import {SolidGeometryDisplayQueue} from './solidGeometryDisplayQueue'
const geometryWeight=(g:SnapGeometry)=>{
 const curves=new Set(g.segments.flatMap(s=>s.nurbs?[s.nurbs.curve]:[]))
 return g.points.length*80+g.segments.length*176+(g.circles?.length??0)*96+[...curves].reduce((n,c)=>n+128+c.controlPoints.length*80+c.knots.length*8+c.weights.length*8,0)
}
type Job=Extract<MainSolidJob,{kind:'sketchSnaps'}>
/** Local targets depend on geometry, not the sketch name, placement or selection. */
export const sketchSnapKey=(sketch:DirectSketch)=>JSON.stringify({points:sketch.points,closed:sketch.closed,analytic:sketch.analytic,retainedProfile:sketch.retainedProfile})
export class SolidSketchSnapPreparation extends SolidGeometryDisplayQueue<DirectSketch,SnapGeometry,Job>{
 constructor(port:{run(job:Job):Promise<SnapGeometry>;cancel():void},maxEntries=256,maxWeight=64_000_000){
  super(port,sketchSnapKey,sketch=>({kind:'sketchSnaps',sketch}),geometryWeight,maxEntries,maxWeight)
 }
}
