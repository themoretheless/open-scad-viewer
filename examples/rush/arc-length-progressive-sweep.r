// @rush/1
profile = bezier_curve(points: [[1mm,0mm,0mm],[2mm,0mm,0mm]])
path = bezier_curve(points: [[0mm,0mm,0mm],[0mm,0mm,10mm]],weights: [1,2])
show progressive_sweep(profile,path,scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},normal: [1,0,0],spacing: "arc_length",initial_sections: 3,max_sections: 3,length_tolerance: 0.001mm,max_deviation: 0.005mm).nurbs_patches_tessellate(segments: 8)
