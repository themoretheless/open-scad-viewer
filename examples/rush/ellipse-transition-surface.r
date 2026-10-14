// @rush/1
shape = ellipse_transition_surface(start_center: [3mm,4mm,5mm],start_axis_u: [2mm,0mm,0mm],start_axis_v: [0mm,1mm,0mm],end_center: [6mm,7mm,8mm],end_axis_u: [0mm,3mm,0mm],end_axis_v: [1mm,1mm,2mm])
show shape.tessellate(segments_u: 32,segments_v: 16)
