// @rush/1
// Rational NURBS surface; tessellation is display geometry.
show ellipse_arc(center: [0mm,0mm,0mm], axis_u: [12mm,0mm,0mm], axis_v: [0mm,8mm,0mm], start_degrees: 0deg, sweep_degrees: 360deg).surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16, segments_v: 32)
