// @rush/1
// Original two-pole rational line; RMF retained-patch error includes the laws.
profile = bezier_curve(points: [[3mm,-2mm,8mm],[3mm,-2mm,9mm]],weights: [1,2])
path = bezier_curve(points: [[3mm,-2mm,7mm],[6mm,2mm,7mm]],weights: [2,3])
show progressive_sweep(profile,path,
 orientation: "rmf",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,90deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 3,max_sections: 129,max_deviation: 0.01mm
).nurbs_patches_tessellate(segments: 8)
