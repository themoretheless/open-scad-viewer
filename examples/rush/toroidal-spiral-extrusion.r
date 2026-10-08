// @rush/1
profile = toroidal_spiral_curve(center: [0mm,0mm,0mm],major_radius: 3mm,minor_radius: 1mm,major_turns: 0.125,minor_turns: 0.375,major_phase_degrees: 23deg,minor_phase_degrees: 47deg,max_deviation: 0.0001mm)
show profile.surface_extrude([0mm,0mm,1mm]).tessellate(segments_u: 32,segments_v: 8)
