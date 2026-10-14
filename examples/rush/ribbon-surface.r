// @rush/1
path = circle_arc(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 5mm,start_degrees: 0deg,sweep_degrees: 90deg)
width = line_curve(start: [1mm,0mm,0mm],end: [2mm,0mm,0mm])
show path.ribbon_surface(width_law: width,normal: [0,0,1],sections: 32,max_deviation: 0.01mm).tessellate(segments_u: 16,segments_v: 32)
