// @rush/1
path = circle_curve(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 5mm)
radius = bezier_curve(points: [[1mm,0mm,0mm],[2mm,0mm,0mm],[1mm,0mm,0mm]],weights: [1,2,1])
show path.variable_pipe_surface(radius_law: radius,normal: [0,0,1],sections: 32,max_deviation: 0.1mm).tessellate(segments_u: 32,segments_v: 32)
