import type { Value } from '../rushGraphNurbsKernel'
import { bezierNurbsCurve, bezierNurbsSurface, bilinearNurbsPatch, circleNurbsArc, circleNurbsCurve, clampedSplineNurbsCurve, closedSplineNurbsCurve, coneFrustumNurbsSurface, coneNurbsSurface, cylinderNurbsSurface, ellipseNurbsArc, ellipsoidNurbsSurface, ellipticCylinderNurbsSurface, gridSplineNurbsSurface, hermiteNurbsPatch, hyperbolaNurbsCurve, hyperboloidOneSheetNurbsSurface, hyperboloidTwoSheetNurbsSurface, lineNurbsCurve, parabolaNurbsCurve, planeNurbsPatch, polylineNurbsCurve, polynomialGraphNurbsSurface, polynomialNurbsCurve, polynomialNurbsSurface, quadraticNurbsPatch, rationalPolynomialNurbsCurve, rationalPolynomialNurbsSurface, roundPolylineNurbsCurve, sphereNurbsSurface, torusNurbsSurface, transitionPolylineNurbsCurve } from '../../kernel/nurbsConstructors'
import { compileRushGraphNurbs } from '../rushGraphNurbsCompiler'

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
