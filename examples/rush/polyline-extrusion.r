// @rush/1
// Degree-one corners remain C0; closing the path does not cap the surface.
show polyline_curve(points: [[-10mm,-6mm,0mm],[10mm,-6mm,0mm],[6mm,8mm,0mm]], closed: true).surface_extrude([0mm,0mm,12mm]).tessellate(segments_u: 16, segments_v: 32)
