// @rush/1
// Frenet frame from original quadratic path derivatives, with scale and twist.
// Continuous retained-patch error remains separate from Solid body admission.
profile = bezier_curve(points: [[0,1mm,1mm],[0,2mm,1mm]])
path = bezier_curve(points: [[0,0,0],[0.5mm,0,0],[1mm,1mm,0]])
show progressive_sweep(profile,path,
 orientation: "frenet",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,14.32394487827058deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 3,max_sections: 257,max_deviation: 0.1mm
).nurbs_patches_tessellate(segments: 4)
