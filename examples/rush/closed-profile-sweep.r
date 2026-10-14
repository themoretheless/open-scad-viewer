// @rush/1
profile = circle_curve(center: [5mm,0mm,0mm],normal: [0,1,0],radius: 1mm)
path = circle_curve(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 5mm)
show profile_sweep(profile,path,scale: {degree: 2,knots: [0,0,0,1,1,1],values: [1,2,1],weights: [1,2,1]},normal: [0,0,1],sections: 32,max_deviation: 0.15mm).tessellate(segments_u: 16,segments_v: 32)
