import type {SolidMinkowskiOptions,SolidMinkowskiResult} from './solidMinkowski'
import {validG2RunoutResult,type createG2RunoutSurface} from './geometricG2'
import type {AssemblyFrameExport} from './assemblyFrameExport'
import type {AssemblyAnimation} from './assemblyAnimation'
import type {DrawingSheetOptions,DrawingSheet} from './drawingSheets'
import {callGeometryRust} from './geometry/kernel'
import {isBoundaryConnectivity,isMaterialAudit} from './structuralSectionsValidation'
export {isBoundaryConnectivity}
import {profileBridgeChordExpectation,validProfileBridgeChordReport,type ProfileBridgeChordOptions,type ProfileBridgeChordReport} from './nurbsProfileBridgeChords'
import {expressionInspectionExpectation,validExpressionInspection,type ExpressionInspectionOptions,type ExpressionInspectionResult} from './expressionShellInspection'
import {expressionShellExpectation,validExpressionShell,type ExpressionShellOptions,type ExpressionShellResult} from './expressionShell'
import {isSourceBodyRecordPayload} from './sourceBodyArchive'
import {sourceBodyExpectation,validSourceBody,type SourceBodyOptions,type SourceBodyResult} from './sourceBody'
import {contactQualificationExpectation,validContactQualification,type OffsetContactQualificationOptions,type OffsetContactQualification,type ContactQualificationExpectation} from './nurbsOffsetContactQualification'
import {envelopeFitExpectation,validEnvelopeFit,type OffsetEnvelopeFitOptions,type OffsetEnvelopeFit,type EnvelopeFitExpectation} from './nurbsOffsetEnvelopeFit'
import type {OffsetContactBandOptions,OffsetContactBand,TrimmedOffsetContactBandOptions,TrimmedOffsetContactBand,OffsetSourceBoundaryOptions,OffsetSourceBoundary,OffsetContactTangent,OffsetEnvelopeOptions,OffsetEnvelope} from './nurbsSurfaceOffset'
import {wholeWallExpectation,validWholeWall,type WholeWallOptions,type WholeWallResult} from './solidWholeWall'
import {materialWallExpectation,validMaterialWall,type MaterialWallOptions,type MaterialWallResult} from './solidMaterialWall'
import {materialExpectation,validMaterial,type MaterialOptions,type MaterialSegment,type MaterialChord} from './solidMaterialVolume'
import type {solidPartialAnnularPreview} from './solidPartialAnnularPreview'
import {validateProfileIntersectionReport,type ProfileIntersectionReport,type ProfileIntersectionOptions} from './geometry/profileIntersections'
import type {TrimmedOffsetOptions,offsetTrimmedSolidCurve} from './solidTrimmedCurveOffset'
import {validCurveOffsetDiagnostics} from './curveOffsetDiagnostics'
import {solidDistanceExpectation,validSolidDistance,type SolidDistanceOptions,type SolidDistanceResult} from './solidDistance'
import {selfIntersectionExpectation,validSelfIntersection,type SelfIntersection} from './solidSelfIntersection'
import {faceContactExpectation,validFaceContacts,type FaceContacts,type FaceContactLimits} from './solidFaceContacts'
import {boundaryExpectation,validBoundaryAgreement,type BoundaryAgreement} from './solidBoundaryAgreement'
import type {SnapGeometry} from './modelingSnaps'
import {validBodySnapGeometry,validSketchSnapGeometry} from './solidSnapProtocol'
import type {prepareSolidFaceSketch} from './solidFaceSketch'
import type {solidBodyEdges} from './solidBodyEdges'
import type {solidTopology} from './directSolidTools'
import {validSolidTopology} from './solidTopologyProtocol'
import type {NurbsCurve,NurbsCurveDistance} from './nurbsCurve'
import type {BrepProfile} from './geometry/brepProfile'
import type {SolidNurbsSurface} from './solidNurbs'
import type {SurfaceBoundaryOptions,SurfaceBoundaryReport} from './solidSurfaceDiagnostics'
import type {NurbsSurface,NurbsSurfaceDistance} from './nurbsSurface'
import type {PointMeasurement,CurveMeasurement,FaceDistanceOptions,FaceDistanceResult,ShellDistanceOptions,ShellDistanceResult} from './solidMeasurements'
import type {SolidPrimitiveOptions} from './solidPrimitive'
import type {DisplayMesh} from './solidDisplayCache'
import type {NurbsBrep} from './geometry/brep'
import type {SolidBrepToolOptions,SolidBrepToolResult} from './solidBrepTool'
import type {SolidNurbsEditOptions} from './solidNurbsEdit'
import type {SolidCurveOffsetOptions,offsetSolidCurve} from './solidCurveOffset'
import type {SolidPointEditOptions} from './solidPointEdit'
import type {SolidSketchEditOptions} from './solidSketchEdit'
import type {SolidBooleanOptions,SolidBooleanResult} from './solidBoolean'
import type {SolidSceneEditOptions} from './solidSceneEdit'
import type {matchSolidCurve} from './solidCurveMatching'
import type {matchSolidSurface,prepareSolidSurfaceSeams} from './solidSurfaceMatching'
import type {SolidSurfaceBuildOptions,buildSolidSurface} from './solidSurfaceConstruction'
import type {SolidNurbsRefitOptions,refitSolidNurbs} from './solidCurveReduction'
import type {prepareSolidProfile} from './solidProfilePreparation'
import type {SolidProfileEditOptions} from './solidProfileEdit'
import type {SolidBodyEditOptions} from './solidBodyEdit'
import type {SolidRevolveOptions} from './solidRevolve'
import type {DirectExtrusionOptions} from './directExtrusion'
import type {inspectSolidIntersections} from './solidDiagnostics'
import type {PolygonMesh} from './geometry/polygon'
import {parseBondedSolidInput,isBondedSolidResult,type BondedSolidResult} from './bondedSolidProtocol'
import type {StructuralSections,BoundaryConnectivity} from './structuralSections'
import type {MeshData} from '../core/mesh'
import type {CadOptions} from './cadWorkbench'
import type {CadPairReport} from './cadInspection'
import type {DirectBody, DirectSketch, DirectDocument} from './directModeling'
import type {MainOperation, MainParameters} from './mainModeling'
import type {PickHit} from './rendererContracts'
import type {TrussInput, TrussResponse} from './trussAnalysis'
import {isNominalLatticeGraph, type LatticeGraphMesh, type NominalLatticeGraph} from './latticeGraphProtocol'
import type {LighteningOptions} from './solidLightening'
import { arrayOf } from './mainSolidProtocol/mainSolidExpectation'
export { mainSolidExpectation } from './mainSolidProtocol/mainSolidExpectation'
export { mainSolidResult } from './mainSolidProtocol/mainSolidResult'

export type MainSolidJob =
  | {kind:'profileBridgeChords';options:ProfileBridgeChordOptions}
  | {kind:'expressionShellInspect';options:ExpressionInspectionOptions}
  | {kind:'expressionShellRestore';options:ExpressionShellOptions}
  | {kind:'sourceBodyRestore';options:SourceBodyOptions}
  | {kind:'offsetContactQualification';options:OffsetContactQualificationOptions}
  | {kind:'offsetEnvelopeFit';options:OffsetEnvelopeFitOptions}
  | {kind:'offsetEnvelope';options:OffsetEnvelopeOptions}
  | {kind:'offsetSourceBoundary';options:OffsetSourceBoundaryOptions}
  | {kind:'trimmedOffsetContactBand';options:TrimmedOffsetContactBandOptions}
  | {kind:'offsetContactTangent';options:OffsetContactBandOptions}
  | {kind:'offsetContactBand';options:OffsetContactBandOptions}
  | {kind:'sketchSnaps';sketch:DirectSketch}
  | {kind:'bodySnaps';body:DirectBody}
  | {kind:'faceSketch';body:DirectBody;face:number}
  | {kind:'bodyEdges';body:DirectBody}
  | {kind:'topology';mesh:PolygonMesh}
  | {kind:'curveDisplay';curve:NurbsCurve}
  | {kind:'profileDisplay';profile:BrepProfile}
  | {kind:'surfaceMesh';item:SolidNurbsSurface}
  | {kind:'surfaceBoundary';a:NurbsSurface;b:NurbsSurface;options:SurfaceBoundaryOptions}
  | {kind:'selfIntersection';model:NurbsBrep;toleranceUv:number;limits:FaceContactLimits;maxSpans:number;boundaryAudit?:import('./solidSelfIntersection').BoundaryAuditLimits}
  | {kind:'faceContacts';model:NurbsBrep;toleranceUv:number;limits:FaceContactLimits}
  | {kind:'boundaryAgreement';model:NurbsBrep;maxCells:number}
  | {kind:'wholeWall';options:WholeWallOptions}
  | {kind:'materialWall';options:MaterialWallOptions}
  | {kind:'materialSegment'|'materialChord';options:MaterialOptions}
  | {kind:'solidDistance';options:SolidDistanceOptions}
  | {kind:'shellDistance';options:ShellDistanceOptions}
  | {kind:'faceDistance';options:FaceDistanceOptions}
  | {kind:'surfaceDistance';a:NurbsSurface;b:NurbsSurface;toleranceMm:number;maxCells:number}
  | {kind:'curveDistance';a:NurbsCurve;b:NurbsCurve;toleranceMm:number;maxCells:number}
  | {kind:'measureVertices';a:DirectBody;indexA:number;b:DirectBody;indexB:number}
  | {kind:'measureEdge';body:DirectBody;edge:number;parameter:number}
  | {kind:'displayMesh';mesh:PolygonMesh;brep?:NurbsBrep;segments:number}
  | {kind:'restoreDocument';text:string}
  | {kind:'primitive';document:DirectDocument;options:SolidPrimitiveOptions}
  | {kind:'rushGraphImport';document:DirectDocument;text:string;group?:string}
  | {kind:'brepTool';document:DirectDocument;options:SolidBrepToolOptions}
  | {kind:'nurbsEdit';document:DirectDocument;options:SolidNurbsEditOptions}
  | {kind:'curveChainInspection';document:DirectDocument;ids:string[];maxPairs:number}
  | {kind:'trimmedCurveOffset';document:DirectDocument;options:TrimmedOffsetOptions}
  | {kind:'curveOffset';document:DirectDocument;options:SolidCurveOffsetOptions}
  | {kind:'pointEdit';document:DirectDocument;options:SolidPointEditOptions}
  | {kind:'sketchEdit';document:DirectDocument;options:SolidSketchEditOptions}
  | {kind:'boolean';document:DirectDocument;options:SolidBooleanOptions}
  | {kind:'sceneEdit';document:DirectDocument|string;options:SolidSceneEditOptions}
  | {kind:'curveMatch';args:Parameters<typeof matchSolidCurve>}
  | {kind:'surfaceMatch';args:Parameters<typeof matchSolidSurface>}
  | {kind:'seamPrepare';args:Parameters<typeof prepareSolidSurfaceSeams>}
  | {kind:'surfaceBuild';document:DirectDocument;options:SolidSurfaceBuildOptions}
  | {kind:'nurbsRefit';document:DirectDocument;options:SolidNurbsRefitOptions}
  | {kind:'profileIntersections';loops:NurbsCurve[][];options:Required<ProfileIntersectionOptions>}
  | {kind:'profilePrepare';document:DirectDocument;ids:string[];tolerance:number}
  | {kind:'profileEdit';document:DirectDocument;options:SolidProfileEditOptions}
  | {kind:'partialAnnularPreview';body:DirectBody;edge:number;radius:number}
  | {kind:'bodyEdit';document:DirectDocument;options:SolidBodyEditOptions}
  | {kind:'revolve'; document:DirectDocument; options:SolidRevolveOptions}
  | {kind:'extrusion'; document:DirectDocument; options:DirectExtrusionOptions}
  | {kind:'main'; meshes:MeshData[]; selected:number; hit:PickHit|null; operation:MainOperation; parameters:MainParameters}
  | {kind:'cad'; document:DirectDocument; options:CadOptions}
  | {kind:'inspect'; bodies:DirectBody[]}
  | {kind:'meshContacts'; mesh:PolygonMesh;maxWork?:number;maxContacts?:number}
  | {kind:'truss'; model:TrussInput}
  | {kind:'bondedSolid';inputJson:string}
  | {kind:'structuralSections'; mesh:LatticeGraphMesh; axis:'x'|'y'|'z'; stations:number[]}
  | {kind:'latticeGraph'; mesh:LatticeGraphMesh; options:LighteningOptions}
  | {kind:'g2RunoutSurface';source:NurbsSurface;plane:NurbsSurface;targetPcurve:NurbsCurve;options:Parameters<typeof createG2RunoutSurface>[3]}
  | {kind:'transaction';document:DirectDocument;script:string}
  | {kind:'drawingPdf';sheets:DrawingSheet[]}
  | {kind:'drawingSheets';bodies:DirectBody[];options:DrawingSheetOptions}
  | {kind:'animationFrames';document:DirectDocument;animation:AssemblyAnimation;fps:number}
  | {kind:'animationSample';document:DirectDocument;animation:AssemblyAnimation;time:number}
  | {kind:'minkowski3d';document:DirectDocument;options:SolidMinkowskiOptions}
/** Exhaustive registry: adding a typed job must also admit it at the worker boundary. */
export const MAIN_SOLID_JOB_KINDS:Readonly<Record<MainSolidJob['kind'],true>>={
 profileBridgeChords:true,
 expressionShellInspect:true,
 expressionShellRestore:true,
 sourceBodyRestore:true,
 offsetContactQualification:true,
 offsetEnvelopeFit:true,
 offsetEnvelope:true,
 offsetContactTangent:true,
 offsetSourceBoundary:true,
 trimmedOffsetContactBand:true,
 offsetContactBand:true,
 wholeWall:true,
 materialWall:true,
 materialSegment:true,
 materialChord:true,
 solidDistance:true,
 selfIntersection:true,
 faceContacts:true,
 boundaryAgreement:true,
 shellDistance:true,
 faceDistance:true,
 surfaceDistance:true,
 curveDistance:true,
 sketchSnaps:true,
 bodySnaps:true,
 faceSketch:true,
 bodyEdges:true,
 topology:true,
 curveDisplay:true,
 profileDisplay:true,
 surfaceMesh:true,
 surfaceBoundary:true,
 measureVertices:true,
 measureEdge:true,
 primitive:true,
 rushGraphImport:true,
 displayMesh:true,
 restoreDocument:true,
 brepTool:true,
 curveChainInspection:true,
 trimmedCurveOffset:true,
 curveOffset:true,
 nurbsEdit:true,
 pointEdit:true,
 sketchEdit:true,
 boolean:true,
 sceneEdit:true,
 curveMatch:true,
 surfaceMatch:true,
 seamPrepare:true,
 surfaceBuild:true,
 nurbsRefit:true,
 profileIntersections:true,
 profilePrepare:true,
 profileEdit:true,
 bodyEdit:true,
 partialAnnularPreview:true,
 revolve:true,
 extrusion:true,
 main:true,
 cad:true,
 inspect:true,
 meshContacts:true,
 truss:true,
 latticeGraph:true,
 structuralSections:true,
 bondedSolid:true,
g2RunoutSurface:true,
transaction:true,
drawingPdf:true,
drawingSheets:true,
animationFrames:true,
animationSample:true,
minkowski3d:true,}
export interface MainSolidResults {profileBridgeChords:ProfileBridgeChordReport;expressionShellInspect:ExpressionInspectionResult;expressionShellRestore:ExpressionShellResult;sourceBodyRestore:SourceBodyResult;offsetContactQualification:OffsetContactQualification;offsetEnvelopeFit:OffsetEnvelopeFit;offsetEnvelope:OffsetEnvelope;offsetContactTangent:OffsetContactTangent;offsetSourceBoundary:OffsetSourceBoundary;trimmedOffsetContactBand:TrimmedOffsetContactBand;offsetContactBand:OffsetContactBand;wholeWall:WholeWallResult;materialWall:MaterialWallResult;materialSegment:MaterialSegment;materialChord:MaterialChord;partialAnnularPreview:ReturnType<typeof solidPartialAnnularPreview>;profileIntersections:ProfileIntersectionReport;trimmedCurveOffset:ReturnType<typeof offsetTrimmedSolidCurve>;curveChainInspection:import('./curveOffsetDiagnostics').CurveOffsetDiagnostics;solidDistance:SolidDistanceResult;selfIntersection:SelfIntersection;faceContacts:FaceContacts;boundaryAgreement:BoundaryAgreement;shellDistance:ShellDistanceResult;faceDistance:FaceDistanceResult;surfaceDistance:NurbsSurfaceDistance;curveDistance:NurbsCurveDistance;sketchSnaps:SnapGeometry;bodySnaps:SnapGeometry;faceSketch:ReturnType<typeof prepareSolidFaceSketch>;bodyEdges:ReturnType<typeof solidBodyEdges>;topology:ReturnType<typeof solidTopology>;curveDisplay:number[][];profileDisplay:[number,number][][];surfaceMesh:PolygonMesh;surfaceBoundary:SurfaceBoundaryReport;measureVertices:PointMeasurement;measureEdge:CurveMeasurement;primitive:DirectDocument;rushGraphImport:DirectDocument;displayMesh:DisplayMesh;restoreDocument:DirectDocument;brepTool:SolidBrepToolResult;nurbsEdit:DirectDocument;curveOffset:ReturnType<typeof offsetSolidCurve>;pointEdit:DirectDocument;sketchEdit:DirectDocument;boolean:SolidBooleanResult;sceneEdit:DirectDocument;curveMatch:ReturnType<typeof matchSolidCurve>;surfaceMatch:ReturnType<typeof matchSolidSurface>;seamPrepare:ReturnType<typeof prepareSolidSurfaceSeams>;surfaceBuild:ReturnType<typeof buildSolidSurface>;nurbsRefit:ReturnType<typeof refitSolidNurbs>;profilePrepare:ReturnType<typeof prepareSolidProfile>;profileEdit:DirectDocument;bodyEdit:DirectDocument;revolve:DirectDocument;extrusion:DirectDocument;meshContacts:ReturnType<typeof inspectSolidIntersections>;bondedSolid:BondedSolidResult; main:DirectDocument; cad:DirectDocument; inspect:CadPairReport[]; truss:TrussResponse; latticeGraph:NominalLatticeGraph; structuralSections:StructuralSections;minkowski3d:SolidMinkowskiResult;
g2RunoutSurface:ReturnType<typeof createG2RunoutSurface>;
animationFrames:AssemblyFrameExport;
drawingPdf:Uint8Array;
animationSample:{document:DirectDocument;camera:{yaw:number;pitch:number}};
drawingSheets:DrawingSheet[];
transaction:DirectDocument;}
export type MainSolidRequest = {version:1; id:number; job:MainSolidJob; traceTiming?:boolean}
export type MainSolidResponse = {version:1; id:number; kind:MainSolidJob['kind']; timing?:{warmupMs:number;executeMs:number;prepareMs:number}} & (
  | {ok:true; result:MainSolidResults[keyof MainSolidResults]}
  | {ok:false; error:{name:string; message:string; code?:string}}
)

export const finite = (v:unknown): v is number => typeof v==='number' && Number.isFinite(v)
/** Mesh buffers arrive as typed views after the worker transfer; plain arrays from JSON. */
export const isNumericSequence=(v:unknown):v is ArrayLike<number>=>
  Array.isArray(v)||(ArrayBuffer.isView(v)&&!(v instanceof DataView))
export const vector = (v:unknown) => arrayOf(v,3,finite)

export type MainSolidExpectation =
  | ({kind:'profileBridgeChords'}&ReturnType<typeof profileBridgeChordExpectation>)
  | ({kind:'expressionShellInspect'}&ReturnType<typeof expressionInspectionExpectation>)
  | ({kind:'expressionShellRestore'}&ReturnType<typeof expressionShellExpectation>)
  | ({kind:'sourceBodyRestore'}&ReturnType<typeof sourceBodyExpectation>)
  | ({kind:'offsetContactQualification'}&ContactQualificationExpectation)
  | ({kind:'offsetEnvelopeFit'}&EnvelopeFitExpectation)
  | {kind:'offsetEnvelope';tangent:Extract<MainSolidExpectation,{kind:'offsetContactTangent'}>;radiusMm:number;maxCells:number}
  | {kind:'offsetContactBand';fixedAxis:0|1;domains:[[[number,number],[number,number]],[[number,number],[number,number]]]}
  | {kind:'offsetContactTangent';fixedAxis:0|1;domains:[[[number,number],[number,number]],[[number,number],[number,number]]]}
  | {kind:'trimmedOffsetContactBand';fixedAxis:0|1;domains:[[[number,number],[number,number]],[[number,number],[number,number]]];maxPairs:number;maxCells:number;maxDomainCells:number;loopCounts:[number,number]}
  | {kind:'offsetSourceBoundary';trim:{kind:'trimmedOffsetContactBand';fixedAxis:0|1;domains:[[[number,number],[number,number]],[[number,number],[number,number]]];maxPairs:number;maxCells:number;maxDomainCells:number;loopCounts:[number,number]};addresses:[number,number,number][];toleranceMm:number;maxExactWork:number;maxAgreementCells:number}
  | {kind:'partialAnnularPreview';id:string;bodyIdentity:string|undefined}
  | {kind:'trimmedCurveOffset';createdId:string;fillRule:'nonzero'|'evenodd';toleranceMm:number;intersectionToleranceMm:number}
  | {kind:'displayMesh';triangles:number}
  | ({kind:'wholeWall'}&ReturnType<typeof wholeWallExpectation>)
  | ({kind:'materialWall'}&ReturnType<typeof materialWallExpectation>)
  | ({kind:'materialSegment'|'materialChord'}&ReturnType<typeof materialExpectation>)
  | ({kind:'solidDistance'}&ReturnType<typeof solidDistanceExpectation>)
  | ({kind:'selfIntersection'}&ReturnType<typeof selfIntersectionExpectation>)
  | ({kind:'faceContacts'}&ReturnType<typeof faceContactExpectation>)
  | ({kind:'boundaryAgreement'}&ReturnType<typeof boundaryExpectation>)
  | {kind:'shellDistance';domains:[number,number][][][];toleranceMm:number;toleranceUv:number;maxCells:number;maxDomainCells:number}
  | {kind:'faceDistance';domains:[number,number][][];toleranceMm:number;toleranceUv:number;maxCells:number;maxDomainCells:number}
  | {kind:'surfaceDistance';domains:[number,number][][];toleranceMm:number;maxCells:number}
  | {kind:'curveDistance';dimension:number;domains:[number,number][];toleranceMm:number;maxCells:number}
  | {kind:'sketchSnaps';maxPoints:number;maxSegments:number;maxCircles:number}
  | {kind:'bodySnaps';maxPoints:number;maxSegments:number}
  | {kind:'bodyEdges';vertices:number;edges:number;exact:boolean}
  | {kind:'topology';vertices:number;triangles:number}
  | {kind:'curveChainInspection';segments:number;maxPairs:number}
  | {kind:'curveDisplay';dimension:number}
  | {kind:'profileDisplay';counts:number[]}
  | {kind:'profileIntersections';loops:NurbsCurve[][];options:Required<ProfileIntersectionOptions>}
  | {kind:'profilePrepare';chainSegments?:number[]}
  | {kind:'surfaceMesh';u:number;v:number}
  | {kind:'surfaceBoundary';samples:number}
  | {kind:'brepTool';id:string;operation:'mass'|'mesh'|'display'}
  | {kind:'meshContacts';triangles:number;maxWork:number;maxContacts:number}
  | {kind:'truss'; nodes:number; members:number}
  | {kind:'structuralSections';axis:'x'|'y'|'z';stations:number[]}
  | {kind:'bondedSolid';nodes:number;tets:number;bonds:number}
  | {kind:Exclude<MainSolidJob['kind'],'profileBridgeChords'|'expressionShellInspect'|'expressionShellRestore'|'minkowski3d'|'g2RunoutSurface'|'surfaceBuild'|'sourceBodyRestore'|'offsetContactQualification'|'offsetEnvelopeFit'|'offsetEnvelope'|'offsetContactTangent'|'offsetSourceBoundary'|'trimmedOffsetContactBand'|'offsetContactBand'|'wholeWall'|'materialWall'|'materialSegment'|'materialChord'|'trimmedCurveOffset'|'curveChainInspection'|'solidDistance'|'selfIntersection'|'faceContacts'|'boundaryAgreement'|'shellDistance'|'faceDistance'|'surfaceDistance'|'curveDistance'|'sketchSnaps'|'bodySnaps'|'bodyEdges'|'topology'|'curveDisplay'|'profileDisplay'|'surfaceMesh'|'surfaceBoundary'|'displayMesh'|'brepTool'|'truss'|'structuralSections'|'bondedSolid'|'meshContacts'|'profilePrepare'|'profileIntersections'|'partialAnnularPreview'>}
  | {kind:'minkowski3d';createdId:string;bodyIds:string[];sourceSizes:[number,number][];sourceKinds:('brep'|'mesh')[]}
  | {kind:'g2RunoutSurface';maxCells:number;maxExactWork:number;maxPatches:number;tolerances:[number,number,number]}
  | {kind:'surfaceBuild';runoutId?:string}
