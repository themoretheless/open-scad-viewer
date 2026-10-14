// @rush/1
profile = torus_knot_curve(center: [0mm,0mm,0mm],major_radius: 3mm,minor_radius: 1mm,p: 2,q: 3,major_phase_degrees: 0deg,minor_phase_degrees: 0deg,max_deviation: 0.0001mm)
show profile.surface_extrude_patches([0mm,0mm,1mm]).nurbs_patches_tessellate(segments: 8)
