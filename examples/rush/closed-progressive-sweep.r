// @rush/1
profile = bezier_curve(points: [[5mm,0mm,-1mm],[5mm,0mm,1mm]])
path = circle_curve(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 5mm)
show progressive_sweep(profile,path,scale: {degree: 2,knots: [0,0,0,1,1,1],values: [1,2,1],weights: [1,2,1]},twist: {degree: 1,knots: [0,0,1,1],values: [0deg,360deg],weights: [1,1]},normal: [0,0,1],initial_sections: 5,max_sections: 257,max_deviation: 0.02mm).nurbs_patches_tessellate(segments: 8)
