// @rush/1
profile = clamped_spline_curve(points: [[-12mm,0mm,0mm],[0mm,8mm,0mm],[15mm,0mm,0mm]],parameters: [-2,0,4],start_tangent: [6mm,0mm,0mm],end_tangent: [3mm,-2mm,0mm])
show profile.surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16,segments_v: 32)
