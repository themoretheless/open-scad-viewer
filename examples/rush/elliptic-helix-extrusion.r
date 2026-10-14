// @rush/1
profile = elliptic_helix_curve(center: [0mm,0mm,0mm],radius_x: 5mm,radius_y: 3mm,height: 12mm,turns: 0.25,phase_degrees: 0deg,max_deviation: 0.0001mm)
show profile.surface_extrude([0mm,0mm,1mm]).tessellate(segments_u: 32,segments_v: 8)
