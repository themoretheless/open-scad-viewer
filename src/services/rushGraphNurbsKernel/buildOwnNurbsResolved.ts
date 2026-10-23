import {readMiterAffineReplayRouting} from '../geometry/brep'
import {packNativeGeometrySnapshot} from '../nativeGeometrySnapshot'
import {readNativeSweepPlanarCapFaces} from '../geometry/brep'
import type {StationResolver} from '../rushGraphNurbsKernel'
import { createOwnNurbsResolver } from './createOwnNurbsResolver'
import { PendingProgressiveBody, PendingProgressiveMiter, PendingProgressiveSweep } from '../rushGraphNurbsKernel'
import type { BodyArguments, BodyResolver, Mesh, MiterArguments, MiterResolver, OwnNurbsRequest, SweepArguments, SweepResolver, Value } from '../rushGraphNurbsKernel'
import { booleanNurbsBrep, chamferNurbsBrepEdges, createBrepBox, createBrepCylinder, createBrepFrustum, createBrepGear, createBrepSphere, createBrepTorus, createBrepTube, createCappedBrepLoftSurfaces, createCappedBrepLoftWithCaps, createMiterBrepProfileBody, createNaturalBrepSectionLoft, createProgressiveBrepProfileBody, createProgressiveMiterBrepProfileBody, extrudeBrepCurves, extrudeBrepPolygon, filletNurbsBrepEdges, reconstructCertifiedMiterStations, revolveBrepProfile, tessellateNurbsBrep, transformCertifiedMiterBody, transformNurbsBrep } from '../geometry/brep'
import type { CertifiedMiterBoundaryOwner, NurbsBrep } from '../geometry/brep'
import { elevateNurbsCurve, evaluateNurbsCurve, insertNurbsKnot, nurbsCurveBounds, reverseNurbsCurve, transformNurbsCurve, trimNurbsCurve, validateNurbsCurve } from '../nurbsCurve'
import type { NurbsCurve } from '../nurbsCurve'
import { elevateNurbsSurface, evaluateNurbsSurface, insertNurbsSurfaceKnot, isoNurbsCurve, nurbsSurfaceBounds, reverseNurbsSurface, transformNurbsSurface, transformNurbsSurfacePatches, trimNurbsSurface, validateNurbsSurface } from '../nurbsSurface'
import type { NurbsSurface } from '../nurbsSurface'
import { evaluateSdf, tessellateSdfGpuAware as tessellateSdf } from '../geometry/sdf'
import type { SdfField } from '../geometry/sdf'
import { approximateArchimedeanSpiralNurbsCurve, approximateCatenaryNurbsCurve, approximateCatenoidNurbsPatches, approximateCatenoidNurbsSurface, approximateClothoidNurbsCurve, approximateConicalHelixNurbsCurve, approximateCycloidNurbsCurve, approximateEllipticHelixNurbsCurve, approximateEpicycloidNurbsCurve, approximateHelicoidNurbsPatches, approximateHelicoidNurbsSurface, approximateHelixNurbsCurve, approximateHypocycloidNurbsCurve, approximateInvoluteNurbsCurve, approximateLissajousNurbsCurve, approximateLogarithmicSpiralNurbsCurve, approximateScrewNurbsSurfaces, approximateSphericalSpiralNurbsCurve, approximateToroidalSpiralNurbsCurve, approximateTorusKnotNurbsCurve, approximateTrochoidNurbsCurve, approximateVariablePitchHelixNurbsCurve, autoGuidedLoftNurbsCurves, bezierNurbsCurve, bezierNurbsSurface, bilinearNurbsPatch, boundaryFillNurbsSurfaces, checkedPipeNurbsSurface, checkedProfileSweepNurbsSurface, checkedRibbonNurbsSurface, checkedVariablePipeNurbsSurface, circleNurbsArc, circleNurbsCurve, circleRectangleTransitionNurbsPatches, circleTransitionNurbsSurface, clampedLoftNurbsCurves, clampedSplineNurbsCurve, closedLoftNurbsCurves, closedSplineNurbsCurve, composeNurbsCurves, coneFrustumNurbsSurface, coneNurbsSurface, controlTangentLoftNurbsCurves, coonsNurbsPatch, cylinderNurbsSurface, ellipseNurbsArc, ellipseTransitionNurbsSurface, ellipsoidNurbsSurface, ellipticCylinderNurbsSurface, extrudeNurbsCurve, extrudeNurbsCurvePatches, formulaNurbsCurve, formulaNurbsSurface, framedSweepNurbsCurve, gordonNurbsSurface, gridSplineNurbsSurface, guidedLoftNurbsCurves, hermiteNurbsCurve, hermiteNurbsPatch, hyperbolaNurbsCurve, hyperboloidOneSheetNurbsSurface, hyperboloidTwoSheetNurbsSurface, lineNurbsCurve, loftAlignedNurbsCurves, loftNurbsCurves, matchNurbsLoftEnds, naturalLoftNurbsCurves, naturalSplineNurbsCurve, parabolaNurbsCurve, planeNurbsPatch, polylineNurbsCurve, polynomialGraphNurbsSurface, polynomialNurbsCurve, polynomialNurbsSurface, progressiveSweepNurbsProfiles, quadraticNurbsPatch, rationalPolynomialNurbsCurve, rationalPolynomialNurbsSurface, revolveNurbsCurve, roundPolylineNurbsCurve, scaledSweepNurbsCurve, sphereNurbsSurface, sweepNurbsCurve, torusNurbsSurface, transitionPolylineNurbsCurve, triangularNurbsPatch, twistSweepNurbsCurve, twoGuideSweepNurbsCurve } from '../nurbsConstructors'
import { booleanPolygonMeshes, extrudePolygonProfile, inspectPolygonMesh, loftPolygonSections, revolvePolygonProfile, sweepPolygonProfile } from '../geometry/polygon'
import { certifyNurbsCurveFoundation, certifyNurbsSurfaceFoundation, createAutoCartesianGuidedLoft, createAutoControlTangentCartesianGuidedLoft, createCartesianGuidedLoft, createControlTangentCartesianGuidedLoft, createMappedNaturalLoft } from '../nurbsFoundation'
import { compileRushGraphNurbs } from '../rushGraphNurbsCompiler'
import { createNativeGeometryArtifact } from '../../core/nativeGeometry'
import { exportMeshFormat, meshExportBase64 } from '../meshExportFormats'
import { exportNurbsStl, tessellateNurbsSurface, thickenNurbsMesh } from '../geometry/tessellation'
import { meshToNurbs, meshToNurbsBrep, meshToSdf, meshToSubdivision, tessellateNurbsPatches } from '../geometry/reconstruction'
import { stringifyMeshJson } from '../meshJson'
import { tessellateSubdivision } from '../geometry/subdivision'

export function buildOwnNurbsResolved(document: unknown, request: OwnNurbsRequest,resolveSweep?:SweepResolver,resolveBody?:BodyResolver,resolveMiter?:MiterResolver,resolveStations?:StationResolver) {
    if (!['build', 'evaluate', 'export'].includes(request.action) || (request.evaluations?.length ?? 0) > 64)
        throw new Error('Invalid NURBS request or evaluation budget exceeded.');
    const compiled = compileRushGraphNurbs(document), nodes = new Map(compiled.resolved_document.nodes.map(n => [n.id, n]));
    const { get, cache, constructionReports, reconstructionReports, exactMiterPlacements } = createOwnNurbsResolver(nodes, request, resolveSweep, resolveBody, resolveMiter, resolveStations)
    const needCurve = (key: string) => { const result = get(key); if (result.kind !== 'curve')
        throw new Error(`Expected curve at ${key}.`); return result.data; };
    const root = get(compiled.document.root);
    const definitions = Object.fromEntries([...cache].filter(([, v]) => v.kind !== 'mesh').map(([id, v]) => [id, { kind: v.kind, ...v.data }]));
    const meshBounds = root.kind === 'mesh' && root.data.positions.length > 0 ? { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] } : null;
    if (root.kind === 'mesh' && meshBounds)
        for (let i = 0; i < root.data.positions.length; i++) {
            const axis = i % 3, v = root.data.positions[i];
            meshBounds.min[axis] = Math.min(meshBounds.min[axis], v);
            meshBounds.max[axis] = Math.max(meshBounds.max[axis], v);
        }
    const patchBounds=root.kind==='patches'&&root.data.patches.length>0?{min:[Infinity,Infinity,Infinity],max:[-Infinity,-Infinity,-Infinity]}:null;
    if(root.kind==='patches'&&patchBounds){
        for(const patch of root.data.patches){
            const bounds=nurbsSurfaceBounds(patch);
            for(let axis=0;axis<3;axis++){
                patchBounds.min[axis]=Math.min(patchBounds.min[axis],bounds.min[axis]);
                patchBounds.max[axis]=Math.max(patchBounds.max[axis],bounds.max[axis]);
            }
        }
    }
    const extendedGeometry=compiled.document.nodes.some(n=>n.op.startsWith('polygon_')||n.op==='subdivision'||n.op==='triangle_mesh'||n.op.startsWith('mesh_to_')||n.op==='mesh_fit_nurbs'||n.op.startsWith('sdf_'));
    const foundationCertificate = root.kind === 'curve' ? certifyNurbsCurveFoundation(root.data)
        : root.kind === 'surface' ? certifyNurbsSurfaceFoundation(root.data) : null;
    const report = { ...(Object.keys(constructionReports).length?{construction:constructionReports}:{}), ...(Object.keys(reconstructionReports).length?{reconstruction:reconstructionReports}:{}), bounds_scope: foundationCertificate ? 'outward-rounded rational span hulls' : root.kind === 'mesh' ? 'derived mesh vertices' : 'conservative control hull', kernel: extendedGeometry?'own-rust-geometry':'own-rust-nurbs', polygon_core: 'own-rust-polygons', geometry_authority: extendedGeometry?'source geometry definitions':'rational control data', root: compiled.document.root, root_kind: root.kind, bounds: root.kind === 'curve' ? nurbsCurveBounds(root.data) : root.kind === 'surface' ? nurbsSurfaceBounds(root.data) : root.kind === 'patches' ? patchBounds : meshBounds, foundation_certificate: foundationCertificate, foundation_successor: foundationCertificate ? {version:'nurbs-ss/1',curve_projection:'Bernstein stationary isolation',surface_projection:'2d outward hull subdivision with Krawczyk/interval uniqueness',normal_regularity:'recursive sub-knot-cell homogeneous cone localization',simplification:'adaptive Lipschitz rollback certified',periodic_editing:'wrapped cyclic collocation with C0/C1/C2 seam evidence',reparameterization:'piecewise positive rational monotone with nested/periodic exact control-net materialization',fitting:'ranked bounded approximate-only with cloud Hausdorff enclosure',intersection:'certified general CC/CS/SS with Bernstein/Krawczyk isolation, BranchGraph, CoedgeTrim maps; Boolean mutation deferred'} : null, definitions, mesh: root.kind === 'mesh' ? root.data.report : null, printability: 'unknown', error_bound_certified: foundationCertificate !== null && Object.keys(constructionReports).length === 0 };
    const evaluations = (request.evaluations ?? []).map(q => { const v = get(q.node); if (v.kind === 'curve')
        return { node: q.node, ...evaluateNurbsCurve(v.data, q.u) }; if (v.kind === 'surface' && q.v !== undefined)
        return { node: q.node, ...evaluateNurbsSurface(v.data, q.u, q.v) }; throw new Error('Evaluation requires a curve or surface, and v for a surface.'); });
    const base = { ok: true, document_sha256: compiled.document_sha256, execution_target: 'own-nurbs', automatic_fallback: false, report, evaluations };
    if (request.action === 'export') {
        if (request.format === 'json')
            return { ...base, artifact: { format: 'json', mime_type: 'application/json', text: stringifyMeshJson(compiled.document, 2) } };
        if (!request.format || root.kind !== 'mesh')
            throw new Error('STL export requires a closed tessellated mesh. Export JSON to retain native NURBS definitions.');
        if (request.format !== 'stl') { const artifact = exportMeshFormat(root.data, request.format); return {...base, artifact: {format: request.format, mime_type: artifact.mimeType, extension: artifact.extension, base64: meshExportBase64(artifact.data)}}; }
        const text = exportNurbsStl(root.data);
        if (text.length > 4 * 1024 * 1024)
            throw new Error('STL export exceeds 4 MiB.');
        return { ...base, artifact: { format: 'stl', mime_type: 'model/stl', text } };
    }
    if(request.action==='build' && request.display) {
        const {segments,subdivisionLevels}=request.display;
        if(!Number.isInteger(segments)||segments<1||segments>32||!Number.isInteger(subdivisionLevels)||subdivisionLevels<0||subdivisionLevels>4)throw new Error('Invalid display tessellation policy');
        let mesh:Mesh|undefined=root.kind==='mesh'?root.data:undefined;
        if(root.kind==='surface')mesh=tessellateNurbsSurface(root.data,{segmentsU:segments,segmentsV:segments});
        if(root.kind==='brep')mesh=tessellateNurbsBrep(root.data,segments);
        if(root.kind==='subdivision')mesh=tessellateSubdivision(root.data,subdivisionLevels);
        if(root.kind==='patches')mesh=tessellateNurbsPatches(root.data,segments);
        let sourceNode=compiled.document.root,sourceValue=root;
        const rootNode=nodes.get(sourceNode)!;
        // Only known display conversions preserve source face correspondence.
        // Boolean/thicken/edit results own their mesh; do not invent CAD lineage.
        if(['tessellate','brep_tessellate','subdivision_tessellate','sdf_tessellate','nurbs_patches_tessellate'].includes(rootNode.op)&&'input' in rootNode) {
            sourceNode=rootNode.input;sourceValue=get(sourceNode);
        }
        if(rootNode.op==='subdivision')sourceValue={kind:'subdivision',data:{vertices:rootNode.vertices,faces:rootNode.faces}};
        const boundary=rootNode.op==='tessellate'&&(rootNode.trim||rootNode.trim_curves)?{trim:rootNode.trim??null,trimCurves:rootNode.trim_curves?{outer:needCurve(rootNode.trim_curves.outer),holes:rootNode.trim_curves.holes.map(needCurve)}:null}:null;
        const sourceConstruction=constructionReports[sourceNode];
        // Evidence is snapshot-local and belongs only to this exact source
        // node. Derived/edited nodes do not inherit another body's certificate.
        const sweepEvidence=sourceConstruction && 'volume' in sourceConstruction ? {
            volume:sourceConstruction.volume,
            profileSmoothness:'profileSmoothness' in sourceConstruction?sourceConstruction.profileSmoothness:null,
            boundaryCertificate:'boundaryCertificate' in sourceConstruction?sourceConstruction.boundaryCertificate:null,
            certifiedErrorUpper:'certifiedErrorUpper' in sourceConstruction?sourceConstruction.certifiedErrorUpper:null,
            retainedCapDecomposition:'retainedCapDecomposition' in sourceConstruction?sourceConstruction.retainedCapDecomposition:null,
            retainedDecomposition:'retainedDecomposition' in sourceConstruction?sourceConstruction.retainedDecomposition:null,
            retainedWallErrorUpper:'retainedWallErrorUpper' in sourceConstruction?sourceConstruction.retainedWallErrorUpper:null,
            capParallelism:'capParallelism' in sourceConstruction?sourceConstruction.capParallelism:null,
            capProjection:'capProjection' in sourceConstruction?sourceConstruction.capProjection:null,
            idealCapDomains:'idealCapDomains' in sourceConstruction?sourceConstruction.idealCapDomains:null,
            filledCapErrorUpper:'filledCapErrorUpper' in sourceConstruction?sourceConstruction.filledCapErrorUpper:null,
            boundaryErrorUpper:'boundaryErrorUpper' in sourceConstruction?sourceConstruction.boundaryErrorUpper:null,
            boundaryErrorWithinBudget:'boundaryErrorWithinBudget' in sourceConstruction?sourceConstruction.boundaryErrorWithinBudget:null,
            boundaryErrorBudget:'boundaryErrorWithinBudget' in sourceConstruction&&'budget' in sourceConstruction?sourceConstruction.budget:null,
            sectionCorrection:'sectionCorrection' in sourceConstruction?sourceConstruction.sectionCorrection:null,
            endpointContourErrorUpper:'endpointContourErrorUpper' in sourceConstruction?sourceConstruction.endpointContourErrorUpper:null,
            authoredFramesApplied:'authoredFramesApplied' in sourceConstruction?sourceConstruction.authoredFramesApplied:false,
            orientationGuideApplied:'orientationGuideApplied' in sourceConstruction?sourceConstruction.orientationGuideApplied:false,
            affineLawsApplied:'affineLawsApplied' in sourceConstruction?sourceConstruction.affineLawsApplied:false,
            profileRegularityCertified:'profileRegularityCertified' in sourceConstruction?sourceConstruction.profileRegularityCertified:false,
            wallRegularityCertified:'wallRegularityCertified' in sourceConstruction?sourceConstruction.wallRegularityCertified:false,
            continuousBound:'continuousBound' in sourceConstruction?sourceConstruction.continuousBound:false,
        }:undefined;
        const replayOwner=exactMiterPlacements.get(sourceNode)?.body
        const sweepMiterReplay=replayOwner?readMiterAffineReplayRouting(replayOwner):null
        const sweepPlanarCapFaces=sourceValue.kind==='brep'?readNativeSweepPlanarCapFaces(sourceValue.data):undefined
        const nativeGeometry=createNativeGeometryArtifact(sourceNode,sourceValue.kind,{geometry:sourceValue.data,boundary,...(sweepMiterReplay?{sweepMiterReplay}:{}),...(sweepPlanarCapFaces!==undefined?{sweepPlanarCapFaces}:{}),...(sourceConstruction&&'stationReconstruction' in sourceConstruction?{sweepStationReconstruction:sourceConstruction.stationReconstruction}:{}),...(sourceConstruction&&'sweepMiterReplay' in sourceConstruction&&sourceConstruction.sweepMiterReplay?{sweepMiterReplay:sourceConstruction.sweepMiterReplay}:{}),...(sweepEvidence?{sweepEvidence}: {}),...(sourceValue.kind==='brep'&&sourceConstruction&&'bodyBoundaryEvidence' in sourceConstruction?{sweepBodyBoundaryEvidence:sourceConstruction.bodyBoundaryEvidence}:{}),...(sourceValue.kind==='patches'&&sourceConstruction&&'method' in sourceConstruction&&sourceConstruction.method==='progressive-fourfold-section-refinement'?{sweepPatchEvidence:sourceConstruction}:{})},compiled.document,packNativeGeometrySnapshot);
        return {...base,...(mesh?{mesh}:{}),nativeGeometry};
    }
    return { ...base, ...(request.action === 'build' && root.kind === 'mesh' ? { mesh: root.data } : {}) };
}
