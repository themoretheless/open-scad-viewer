import type {NurbsCurve} from './nurbsCurve'
interface CurveItem {id:string;curve:NurbsCurve}
import type {MainSolidJob} from './mainSolidProtocol'
import {SolidGeometryDisplayQueue} from './solidGeometryDisplayQueue'
export type {GeometryDisplayResult as CurveDisplayResult} from './solidGeometryDisplayQueue'
type Job=Extract<MainSolidJob,{kind:'curveDisplay'}>
export const curveDisplayKey=(item:CurveItem)=>JSON.stringify(item.curve)
export class SolidCurveDisplayQueue extends SolidGeometryDisplayQueue<CurveItem,number[][],Job> {
 constructor(port:{run(job:Job):Promise<number[][]>;cancel():void},maxEntries=256,maxWeight=64_000_000){
  super(port,curveDisplayKey,item=>({kind:'curveDisplay' as const,curve:item.curve}),mesh=>mesh.reduce((n,point)=>n+32+point.length*8,0),maxEntries,maxWeight)
 }
}
