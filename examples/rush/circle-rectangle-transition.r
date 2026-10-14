// @rush/1
shape = circle_rectangle_transition(circle_center: [0mm,0mm,0mm],circle_normal: [0,0,1],circle_seam: [1,0,0],circle_radius: 2mm,rectangle_center: [1mm,2mm,5mm],rectangle_axis_u: [3mm,0mm,0mm],rectangle_axis_v: [0mm,1mm,0mm])
show shape.nurbs_patches_tessellate(segments: 16)
