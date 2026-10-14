// @rush/1
profile = spherical_spiral_curve(center: [0mm,0mm,0mm],radius: 2mm,longitude_turns: 1,latitude_turns: 0.5,longitude_phase_degrees: 31deg,latitude_phase_degrees: -90deg,max_deviation: 0.0001mm)
show profile.surface_extrude_patches([0mm,0mm,0.2mm]).nurbs_patches_tessellate(segments: 8)
