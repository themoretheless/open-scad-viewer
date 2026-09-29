import type {SolidNurbsSurface} from './solidNurbs'
import type {PolygonMesh} from './geometry/polygon'
import type {MainSolidJob} from './mainSolidProtocol'
import {SolidGeometryDisplayQueue} from './solidGeometryDisplayQueue'
export type {GeometryDisplayResult as SurfaceDisplayResult} from './solidGeometryDisplayQueue'
type Job=Extract<MainSolidJob,{kind:'surfaceMesh'}>
export const surfaceDisplayKey=(item:SolidNurbsSurface)=>JSON.stringify({surface:item.surface,u:item.segmentsU,v:item.segmentsV})
export class SolidSurfaceDisplayQueue extends SolidGeometryDisplayQueue<SolidNurbsSurface,PolygonMesh,Job> {
 constructor(port:{run(job:Job):Promise<PolygonMesh>;cancel():void},maxEntries=256,maxWeight=64_000_000){
  super(port,surfaceDisplayKey,item=>({kind:'surfaceMesh' as const,item}),mesh=>mesh.positions.byteLength+mesh.indices.byteLength,maxEntries,maxWeight)
 }
}
