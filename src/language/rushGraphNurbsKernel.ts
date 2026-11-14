import {reconstructCertifiedProgressiveStations} from '../kernel/geometry/brep'
import {type ProgressiveSweepPreview,progressiveSweepNurbsProfiles} from '../kernel/nurbsConstructors';
import {type PolygonProfile} from '../kernel/geometry/polygon';
import {type NurbsSurfaceSet} from '../kernel/geometry/reconstruction';
import type {SubdivisionCage} from '../kernel/geometry/subdivision';
import {type SdfField} from '../kernel/geometry/sdf';
import {createProgressiveMiterBrepProfileBody,type ProgressiveMiterBrepBody,createProgressiveBrepProfileBody,type NurbsBrep} from '../kernel/geometry/brep';
import { type MeshExportFormat } from '../exchange/meshExportFormats';
import { type NurbsCurve } from '../kernel/nurbsCurve';
import { type NurbsSurface } from '../kernel/nurbsSurface';
import { tessellateNurbsSurface } from '../kernel/geometry/tessellation';
export { buildOwnNurbs, buildOwnNurbsAsync } from './rushGraphNurbsKernel/buildOwnNurbs'
export type Mesh = ReturnType<typeof tessellateNurbsSurface>;
export type Value = {kind:'profile';data:PolygonProfile} | {kind:'patches';data:NurbsSurfaceSet} | {kind:'subdivision';data:SubdivisionCage} | {kind:'sdf';data:SdfField} | {kind:'brep';data:NurbsBrep} | {
    kind: 'curve';
    data: NurbsCurve;
} | {
    kind: 'surface';
    data: NurbsSurface;
} | {
    kind: 'mesh';
    data: Mesh;
};
export type OwnNurbsRequest = {
    action: 'build' | 'evaluate' | 'export';
    offsetDeviation?:import('./rushGraphOffsetDeviation').RushOffsetDeviationRequest;
    format?: 'json' | MeshExportFormat;
    /** Viewer-only derived display policy; does not change the authored graph. */
    display?: {segments:number;subdivisionLevels:number};
    evaluations?: Array<{
        node: string;
        u: number;
        v?: number;
    }>;
};
export {collectSdfJobs,type SdfTessellationJob} from './rushGraphSdfJobs'

export type SweepArguments=Parameters<typeof progressiveSweepNurbsProfiles>
export type SweepResolver=(nodeId:string,args:SweepArguments)=>ReturnType<typeof progressiveSweepNurbsProfiles>
export class PendingProgressiveSweep {
 constructor(readonly nodeId:string,readonly args:SweepArguments){}
}
export type BodyArguments=Parameters<typeof createProgressiveBrepProfileBody>
export type BodyResolver=(nodeId:string,args:BodyArguments)=>ReturnType<typeof createProgressiveBrepProfileBody>
export class PendingProgressiveBody {
 constructor(readonly nodeId:string,readonly args:BodyArguments){}
}
export type MiterArguments=Parameters<typeof createProgressiveMiterBrepProfileBody>
export type MiterResolver=(nodeId:string,args:MiterArguments)=>ProgressiveMiterBrepBody
export class PendingProgressiveMiter {constructor(readonly nodeId:string,readonly args:MiterArguments){}}
export interface OwnNurbsBuildControl {
 shouldAbort?:()=>boolean
 onYield?:()=>void
 onSweepPreview?:(nodeId:string,preview:ProgressiveSweepPreview)=>void|Promise<void>
 exportQuality?:import('../parser/types').ExportMeshQuality
}

export type StationArguments=Parameters<typeof reconstructCertifiedProgressiveStations>
export type StationResolver=(nodeId:string,args:StationArguments)=>ReturnType<typeof reconstructCertifiedProgressiveStations>
export class PendingProgressiveStations {constructor(readonly nodeId:string,readonly args:StationArguments){}}
