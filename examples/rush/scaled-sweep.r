// @rush/1
profile = circle_curve(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 5mm)
path = bezier_curve(points: [[0mm,0mm,0mm],[0mm,0mm,15mm],[8mm,0mm,30mm]])
show scaled_sweep(profile,path,origin: [0mm,0mm,0mm],scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]}).tessellate(segments_u: 16,segments_v: 16)
