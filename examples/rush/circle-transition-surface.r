// @rush/1
shape = circle_transition_surface(start_center: [0mm,0mm,0mm],start_normal: [0,0,1],start_seam: [1,0,0],start_radius: 2mm,end_center: [3mm,4mm,5mm],end_normal: [1,0,0],end_seam: [0,1,0],end_radius: 3mm)
show shape.tessellate(segments_u: 32,segments_v: 16)
