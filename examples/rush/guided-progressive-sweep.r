// @rush/1
profile = bezier_curve(points: [[2mm,0mm,0mm],[3mm,0mm,1mm]],weights: [1,2])
path = bezier_curve(points: [[0mm,0mm,0mm],[0mm,0mm,10mm]])
rail = bezier_curve(points: [[1mm,0,0],[0,1mm,10mm]],weights: [1,2])
show progressive_sweep(profile,path,scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},twist: {degree: 1,knots: [0,0,1,1],values: [0deg,180deg],weights: [1,1]},orientation_guide: rail,normal: [1,0,0],initial_sections: 5,max_sections: 257,max_deviation: 0.005mm).nurbs_patches_tessellate(segments: 8)
