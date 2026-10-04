// @rush/1
profile = line_curve(start: [1mm,2mm,3mm],end: [2mm,2mm,3mm])
path = line_curve(start: [0,0,0],end: [0,0,10mm])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,90deg],weights: [1,1]},
 orientation: "authored",
 frame_axis: {degree: 1,knots: [4,4,8,8],values: [[2,0,0],[2,0,0]],weights: [1,1]},
 frame_normal: {degree: 1,knots: [2,2,6,6],values: [[7,1,0],[7,1,0]],weights: [1,1]},
 normal: [1,0,0],initial_sections: 3,max_sections: 129,max_deviation: 0.001mm
).nurbs_patches_tessellate(segments: 4)
