// @rush/1
path = circle_arc(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 5mm,start_degrees: 0deg,sweep_degrees: 90deg)
show path.pipe_surface(radius: 1mm,normal: [0,0,1],sections: 32,max_deviation: 0.005mm).tessellate(segments_u: 32,segments_v: 32)
