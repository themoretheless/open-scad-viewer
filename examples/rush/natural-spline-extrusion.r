// @rush/1
// Natural cubic: site interpolation, automatic tangents, zero endpoint curvature vector.
profile = natural_spline_curve(points: [[-12mm,0mm,0mm],[0mm,8mm,0mm],[15mm,0mm,0mm]],parameters: [-2,0,4])
show profile.surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16,segments_v: 32)
