// @rush/1
shape = helicoid_patches(center: [0mm,0mm,0mm],inner_radius: 1mm,outer_radius: 2mm,height: 8mm,turns: 2,phase_degrees: 31deg,max_deviation: 0.0001mm)
show shape.nurbs_patches_tessellate(segments: 8)
