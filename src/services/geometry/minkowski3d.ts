import {callGeometryRust} from './kernel'
import {normalizePolygonMesh,type PolygonMesh} from './polygon'
import type {NurbsBrep} from './brep'
export interface RoundingTopology {status:'proved';precision:'f32'|'f64';identity:boolean;pairs:number;wholeLinearHomotopy:true}
export interface Minkowski3dSource {positions:number[];indices:number[]}
export interface Minkowski3dCertificate {
 version:1;scope:'represented-positive-filled-polyhedral-minkowski-sum';sources:[string,string]
 vertices:[string,string][][];triangles:[number,number,number][];sourceCells:[number,number];sumCells:number
}
export interface Minkowski3dResult {
 status:'verified';scope:Minkowski3dCertificate['scope'];certificate:Minkowski3dCertificate;mesh:PolygonMesh
 volumeMm3:number;exactVolumeMm3:[string,string];maximumCoordinateRoundingMm:number;curvedSourceErrorCertified:false;displayCoordinatesExact:boolean
 roundingTopology:RoundingTopology;maximumSetDeviationMm:number
}
export interface Minkowski3dProof {version:1;a:Minkowski3dSource;b:Minkowski3dSource;certificate:Minkowski3dCertificate;meshPrecision?:'f32'|'f64'}
export type MinkowskiAuthoredSource={kind:'mesh';mesh:Minkowski3dSource}|{kind:'brep';model:NurbsBrep}
export interface SourceTopologyReport {kind:'radial-homeomorphism';center:[string,string][];radialDegree:1;chartCells:number;nominalTriangles:number;filledSet:'closed-three-ball'}
export type ExactRatio=[string,string]
export type RationalAtlasTopologyReport={kind:'rational-atlas-ambient-isotopy';chartTriangles:number;vertexStars:number;disjointPairs:number;boundaryPairs:number;preservesComplement:true}
export type SourceAtlasTopologyReport=RationalAtlasTopologyReport|{kind:'signed-prismatic-ambient-isotopy';axis:number;levels:ExactRatio[];shells:{shell:number;interval:[ExactRatio,ExactRatio];curves:number[];coefficients:number[]}[];curveParents:(number|null)[];nominalProfileVertices:number;preservesComplement:true}|{kind:'polynomial-shear-ambient-isotopy';axis:number;polynomial:ExactRatio[][];sourceChartCells:number;referenceTriangles:number;homotopySegments:number;checkedPairs:number;preservesComplement:true}
export interface SumTopologyReport {kind:'convex-hole-prismatic-minkowski-homeomorphism';axis:number;interval:[ExactRatio,ExactRatio];holedOperand:number;holes:{hole:number;open:boolean;depth:number;innerCore:[ExactRatio,ExactRatio][];outerCore:[ExactRatio,ExactRatio][]}[];survivingHoles:number}
export type ApproximationTopology={status:'proved';kind:'radial-minkowski-homeomorphism';sourceCenters:[string,string][][];filledSet:'closed-three-ball';boundary:'two-sphere';includesMeshRounding:true}|{status:'proved';kind:'exact-source-minkowski-identity';includesMeshRounding:true}|{status:'proved';kind:'prismatic-minkowski-homeomorphism';axis:number;survivingHoles:number;includesMeshRounding:true}
export interface NurbsMinkowskiCertificate {version:1|2|3|4;scope:'represented-nurbs-filled-minkowski-enclosures';sources:[string,string];depths:[number,number];maximumSourceDeviationMm:number;sourceHomotopyBounds:[[string,string],[string,string]];filledHausdorffBound:[string,string]|null;nominal:Minkowski3dCertificate;inner:Minkowski3dCertificate;outer:Minkowski3dCertificate;sourceTopology?:[SourceTopologyReport|null,SourceTopologyReport|null];sourceAtlasTopology?:[SourceAtlasTopologyReport|null,SourceAtlasTopologyReport|null];sumTopology?:SumTopologyReport|null}
export interface NurbsMinkowskiProof {version:2;a:MinkowskiAuthoredSource;b:MinkowskiAuthoredSource;certificate:NurbsMinkowskiCertificate;meshPrecision:'f32'|'f64'}
export interface NurbsMinkowskiResult extends Omit<Minkowski3dResult,'scope'|'certificate'|'curvedSourceErrorCertified'|'maximumSetDeviationMm'> {scope:NurbsMinkowskiCertificate['scope'];certificate:NurbsMinkowskiCertificate;curvedSourceErrorCertified:true;maximumSetDeviationMm:number|null;maximumSetDeviationExactMm:[string,string]|null;sourceSurfaceDeviationMm:[number,number];nominalIsExactCurvedSum:false;approximationTopology:ApproximationTopology|null;sourceApproximationTopology?:[boolean,boolean]}
export type MinkowskiProof=Minkowski3dProof|NurbsMinkowskiProof
export const minkowskiSource=(mesh:PolygonMesh):Minkowski3dSource=>({positions:Array.from(mesh.positions),indices:Array.from(mesh.indices)})
export function certifyMinkowski3d(a:Minkowski3dSource,b:Minkowski3dSource,meshPrecision:'f32'|'f64'='f64'):Minkowski3dResult {
 const result=callGeometryRust<Minkowski3dResult>('minkowski_3d_certify',{a,b,meshPrecision});normalizePolygonMesh(result.mesh);return result
}
export function verifyMinkowski3d(proof:Minkowski3dProof):Minkowski3dResult {
 const result=callGeometryRust<Minkowski3dResult>('minkowski_3d_verify',{a:proof.a,b:proof.b,certificate:proof.certificate,meshPrecision:proof.meshPrecision??'f64'});normalizePolygonMesh(result.mesh);return result
}
export function certifyNurbsMinkowski(a:MinkowskiAuthoredSource,b:MinkowskiAuthoredSource,maximumSourceDeviationMm:number,meshPrecision:'f32'|'f64'='f64'):NurbsMinkowskiResult {
 const result=callGeometryRust<NurbsMinkowskiResult>('minkowski_nurbs_certify',{a,b,maximumSourceDeviationMm,meshPrecision});normalizePolygonMesh(result.mesh);return result
}
export function verifyMinkowskiProof(proof:MinkowskiProof):Minkowski3dResult|NurbsMinkowskiResult {
 if(proof.version===1)return verifyMinkowski3d(proof)
 const result=callGeometryRust<NurbsMinkowskiResult>('minkowski_nurbs_verify',{a:proof.a,b:proof.b,certificate:proof.certificate,meshPrecision:proof.meshPrecision});normalizePolygonMesh(result.mesh);return result
}

export interface NurbsSourceAtlasCertificate {
 version:1;scope:'represented-nurbs-source-atlas-isotopy';source:string;depth:number
 positions:[ExactRatio,ExactRatio,ExactRatio][];triangles:[number,number,number][];report:RationalAtlasTopologyReport
}
export function certifyNurbsSourceAtlas(model:NurbsBrep,depth:number):NurbsSourceAtlasCertificate {
 return callGeometryRust<{status:'verified';certificate:NurbsSourceAtlasCertificate}>('nurbs_source_atlas_certify',{model,depth}).certificate
}
export function verifyNurbsSourceAtlas(model:NurbsBrep,certificate:NurbsSourceAtlasCertificate):NurbsSourceAtlasCertificate {
 return callGeometryRust<{status:'verified';certificate:NurbsSourceAtlasCertificate}>('nurbs_source_atlas_verify',{model,certificate}).certificate
}
