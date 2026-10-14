// @rush/1
profile = involute_curve(center: [0mm,0mm,0mm],radius: 5mm,start_degrees: 0deg,end_degrees: 60deg,max_deviation: 0.0001mm)
show profile.surface_extrude([0mm,0mm,1mm]).tessellate(segments_u: 32,segments_v: 8)
