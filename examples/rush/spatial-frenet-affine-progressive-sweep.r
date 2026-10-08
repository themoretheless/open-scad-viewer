// @rush/1
// Frenet frame from original spatial cubic path derivatives, with affine scale, center, uniform scale and twist.
// Continuous retained-patch error remains separate from Solid body admission.
profile = bezier_curve(points: [[0,1mm,1mm],[0,2mm,1mm]])
path = bezier_curve(points: [[0,0,0],[0.3333333333333333mm,0,0],[0.6666666666666666mm,0.3333333333333333mm,0],[1mm,1mm,1mm]])
show progressive_sweep(profile,path,
 orientation: "frenet",
 axis_scale: {degree: 1,knots: [7,7,9,9],values: [[1,2,1],[2,1,1]],weights: [1,1]},
 center_law: {degree: 1,knots: [11,11,13,13],values: [[0,0,0],[0.125mm,0.25mm,0]],weights: [1,1]},
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,14.32394487827058deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 3,max_sections: 129,max_deviation: 0.1mm
).nurbs_patches_tessellate(segments: 4)
