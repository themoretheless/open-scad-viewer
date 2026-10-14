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

type NativeNode = ReturnType<typeof compileRushGraphNurbs>['resolved_document']['nodes'][number]
export function buildNativePrimitive(n: NativeNode): Value | undefined {
    switch (n.op) {
                case 'parabola_curve': return {kind:'curve',data:parabolaNurbsCurve(n.center,n.axis_u,n.axis_v,n.start,n.end)};
                case 'hyperbola_curve': return {kind:'curve',data:hyperbolaNurbsCurve(n.center,n.axis_u,n.axis_v,n.start,n.end)};
                case 'elliptic_cylinder_surface': return {kind:'surface',data:ellipticCylinderNurbsSurface(n.center,n.radius_x,n.radius_y,n.height)};
                case 'cone_frustum_surface': return {kind:'surface',data:coneFrustumNurbsSurface(n.center,n.bottom_radius,n.top_radius,n.height)};
                case 'hyperboloid_one_sheet': return {kind:'surface',data:hyperboloidOneSheetNurbsSurface(n.center,n.radii,n.start,n.end)};
                case 'line_curve': return {kind:'curve',data:lineNurbsCurve(n.start,n.end)};
                case 'bezier_curve': return {kind:'curve',data:bezierNurbsCurve(n.points,n.weights)};
                case 'sphere_surface': return {kind:'surface',data:sphereNurbsSurface(n.center,n.radius)};
                case 'cylinder_surface': return {kind:'surface',data:cylinderNurbsSurface(n.center,n.radius,n.height)};
                case 'cone_surface': return {kind:'surface',data:coneNurbsSurface(n.center,n.radius,n.height)};
                case 'plane_patch': return {kind:'surface',data:planeNurbsPatch(n.origin,n.axis_u,n.axis_v)};
                case 'grid_spline_surface': return {kind:'surface',data:gridSplineNurbsSurface(n.points,n.parameters_u,n.parameters_v)};
                case 'hermite_patch': return {kind:'surface',data:hermiteNurbsPatch(n.corners,n.tangent_u,n.tangent_v,n.twist)};
                case 'bilinear_patch': return {kind:'surface',data:bilinearNurbsPatch(n.corners)};
                case 'bezier_surface': return {kind:'surface',data:bezierNurbsSurface(n.points,n.weights)};
                case 'transition_polyline_curve': return {kind:'curve',data:transitionPolylineNurbsCurve(n.points,n.setback,n.closed)};
                case 'round_polyline_curve': return {kind:'curve',data:roundPolylineNurbsCurve(n.points,n.radius,n.closed)};
                case 'polyline_curve': return {kind:'curve',data:polylineNurbsCurve(n.points,n.closed)};
                case 'circle_curve': return {kind:'curve',data:circleNurbsCurve(n.center,n.normal,n.radius)};
                case 'circle_arc': return {kind:'curve',data:circleNurbsArc(n.center,n.normal,n.radius,n.start_degrees,n.sweep_degrees)};
                case 'hyperboloid_two_sheet': return {kind:'surface',data:hyperboloidTwoSheetNurbsSurface(n.center,n.radii,n.start,n.end,n.lower)};
                case 'polynomial_graph': return {kind:'surface',data:polynomialGraphNurbsSurface(n.bounds,n.coefficients)};
                case 'polynomial_curve': return {kind:'curve',data:polynomialNurbsCurve(n.domain,n.coefficients)};
                case 'polynomial_surface': return {kind:'surface',data:polynomialNurbsSurface(n.domain,n.coefficients)};
                case 'rational_polynomial_curve': return {kind:'curve',data:rationalPolynomialNurbsCurve(n.domain,n.coefficients)};
                case 'rational_polynomial_surface': return {kind:'surface',data:rationalPolynomialNurbsSurface(n.domain,n.coefficients)};
                case 'quadratic_patch': return {kind:'surface',data:quadraticNurbsPatch(n.bounds,n.coefficients)};
                case 'ellipse_arc': return {kind:'curve',data:ellipseNurbsArc(n.center,n.axis_u,n.axis_v,n.start_degrees,n.sweep_degrees)};
                case 'ellipsoid_surface': return {kind:'surface',data:ellipsoidNurbsSurface(n.center,n.radii)};
                case 'torus_surface': return {kind:'surface',data:torusNurbsSurface(n.center,n.major_radius,n.radial_radius,n.axial_radius)};
                case 'closed_spline_curve': return {kind:'curve',data:closedSplineNurbsCurve(n.points,n.parameters)};
                case 'clamped_spline_curve': return {kind:'curve',data:clampedSplineNurbsCurve(n.points,n.parameters,n.start_tangent,n.end_tangent)};
        default: return undefined
    }
}
