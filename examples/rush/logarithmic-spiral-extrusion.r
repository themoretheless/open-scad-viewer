// @rush/1
profile = logarithmic_spiral_curve(center: [0mm,0mm,0mm],radius: 5mm,growth: 0.3,start_degrees: 0deg,end_degrees: 60deg,max_deviation: 0.0001mm)
show profile.surface_extrude([0mm,0mm,1mm]).tessellate(segments_u: 32,segments_v: 8)
