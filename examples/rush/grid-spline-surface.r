// @rush/1
panel = grid_spline_surface(points: [[[0mm,0mm,0mm],[0mm,10mm,0mm],[0mm,20mm,0mm]],[[10mm,0mm,0mm],[10mm,10mm,8mm],[10mm,20mm,0mm]],[[30mm,0mm,0mm],[30mm,10mm,0mm],[30mm,20mm,0mm]]],parameters_u: [0,1,3],parameters_v: [0,1,2])
show panel.tessellate(segments_u: 24,segments_v: 24)
