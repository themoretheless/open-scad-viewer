// @rush/1
profile = conical_helix_curve(center: [0mm,0mm,0mm],start_radius: 5mm,end_radius: 2mm,height: 12mm,turns: 0.25,phase_degrees: 0deg,max_deviation: 0.0001mm)
show profile.surface_extrude([0mm,0mm,1mm]).tessellate(segments_u: 32,segments_v: 8)
