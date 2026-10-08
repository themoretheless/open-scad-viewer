// @rush/1
profile = bezier_curve(points: [[1mm,2mm,1mm],[2mm,3mm,1mm]],weights: [1,2])
path = bezier_curve(points: [[0,0,0],[0,0,10mm]])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,90deg],weights: [1,1]},
 axis_scale: {degree: 1,knots: [2,2,6,6],values: [[1,2,3],[2,1,4]],weights: [1,2]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0,0,0],[1mm,-2mm,0.5mm]],weights: [1,1]},
 normal: [1,0,0],max_sections: 257,max_deviation: 0.01mm
).nurbs_patches_tessellate(segments: 8)
