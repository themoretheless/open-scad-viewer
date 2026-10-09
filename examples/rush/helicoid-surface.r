// @rush/1
shape = helicoid_surface(center: [0mm,0mm,0mm],inner_radius: 0mm,outer_radius: 2mm,height: 3mm,turns: 0.25,phase_degrees: 31deg,max_deviation: 0.0001mm)
show shape.tessellate(segments_u: 16,segments_v: 32)
