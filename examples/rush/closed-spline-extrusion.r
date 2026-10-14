// @rush/1
profile = closed_spline_curve(points: [[10mm,0mm,0mm],[0mm,10mm,0mm],[-10mm,0mm,0mm],[0mm,-10mm,0mm],[10mm,0mm,0mm]],parameters: [0,1,2,3,4])
show profile.surface_extrude([0mm,0mm,10mm]).tessellate(segments_u: 16,segments_v: 32)
