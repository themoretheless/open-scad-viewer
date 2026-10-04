// @rush/1
profile = bezier_curve(points: [[5mm,0mm,-1mm],[6mm,0mm,0mm],[5mm,0mm,1mm]],weights: [1,2,1])
path = circle_arc(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 5mm,start_degrees: 0deg,sweep_degrees: 90deg)
show profile_sweep(profile,path,scale: {degree: 1,knots: [2,2,6,6],values: [1,2],weights: [1,1]},normal: [0,0,1],sections: 32,max_deviation: 0.02mm).tessellate(segments_u: 16,segments_v: 32)
