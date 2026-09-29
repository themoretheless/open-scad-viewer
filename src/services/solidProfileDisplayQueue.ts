import type {BrepProfile} from './geometry/brepProfile'
interface ProfileItem {id:string;profile:BrepProfile}
import type {MainSolidJob} from './mainSolidProtocol'
import {SolidGeometryDisplayQueue} from './solidGeometryDisplayQueue'
export type {GeometryDisplayResult as ProfileDisplayResult} from './solidGeometryDisplayQueue'
type Job=Extract<MainSolidJob,{kind:'profileDisplay'}>
export const profileDisplayKey=(item:ProfileItem)=>JSON.stringify(item.profile)
export class SolidProfileDisplayQueue extends SolidGeometryDisplayQueue<ProfileItem,[number,number][][],Job> {
 constructor(port:{run(job:Job):Promise<[number,number][][]>;cancel():void},maxEntries=256,maxWeight=64_000_000){
  super(port,profileDisplayKey,item=>({kind:'profileDisplay' as const,profile:item.profile}),mesh=>mesh.reduce((n,loop)=>n+32+loop.length*48,0),maxEntries,maxWeight)
 }
}
