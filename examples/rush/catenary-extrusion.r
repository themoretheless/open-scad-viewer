// @rush/1
profile = catenary_curve(center: [0mm,0mm,0mm],scale: 5mm,start_x: -2mm,end_x: 3mm,max_deviation: 0.0001mm)
show profile.surface_extrude([0mm,0mm,1mm]).tessellate(segments_u: 32,segments_v: 8)
