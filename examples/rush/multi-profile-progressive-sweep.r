// @rush/1
outer = bezier_curve(points: [[2mm,0,0],[2mm,2mm,0]])
adjacent = bezier_curve(points: [[2mm,2mm,0],[0,2mm,0]])
path = bezier_curve(points: [[0,0,0],[0,0,10mm]])
show progressive_sweep(outer,adjacent,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,90deg],weights: [1,1]},
 normal: [1,0,0],initial_sections: 3,max_sections: 257,max_deviation: 0.001mm
).nurbs_patches_tessellate(segments: 8)
