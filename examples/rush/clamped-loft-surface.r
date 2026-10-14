// @rush/1
a = circle_arc(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 10mm,start_degrees: 0deg,sweep_degrees: 90deg)
b = circle_arc(center: [0mm,0mm,8mm],normal: [0,0,1],radius: 15mm,start_degrees: 0deg,sweep_degrees: 90deg)
c = circle_arc(center: [0mm,0mm,30mm],normal: [0,0,1],radius: 8mm,start_degrees: 0deg,sweep_degrees: 90deg)
show clamped_loft_surface(a,b,c,parameters: [0,1,3],start_tangent: [0mm,0mm,8mm],end_tangent: [0mm,0mm,12mm]).tessellate(segments_u: 24,segments_v: 32)
