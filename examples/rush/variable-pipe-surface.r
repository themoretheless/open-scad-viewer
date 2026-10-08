// @rush/1
path = circle_arc(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 5mm,start_degrees: 0deg,sweep_degrees: 90deg)
radius = line_curve(start: [1mm,0mm,0mm],end: [2mm,0mm,0mm])
show path.variable_pipe_surface(radius_law: radius,normal: [0,0,1],sections: 32,max_deviation: 0.02mm).tessellate(segments_u: 32,segments_v: 32)
