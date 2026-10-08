// @rush/1
profile = clothoid_curve(center: [0mm,0mm,0mm],length: 2mm,start_curvature: 0 / 1mm,end_curvature: 2 / 1mm,phase_degrees: 0deg,max_deviation: 0.0001mm)
show profile.surface_extrude_patches([0mm,0mm,0.2mm]).nurbs_patches_tessellate(segments: 8)
