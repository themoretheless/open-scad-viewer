// @rush/1
profile = bezier_curve(points: [[1mm,0mm,0mm],[2mm,0mm,1mm]],weights: [1,2])
show profile.screw_surface(origin: [0mm,0mm,0mm],axis: [0,0,1],height: 4mm,turns: 1,phase_degrees: 23deg,max_deviation: 0.0001mm).nurbs_patches_tessellate(segments: 8)
