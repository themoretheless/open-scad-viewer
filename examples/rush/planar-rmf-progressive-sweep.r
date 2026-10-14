// @rush/1
// Original planar RMF with a perpendicular constant Bishop normal.
// Retained-patch proof; this open surface has no native Solid ownership.
profile = bezier_curve(points: [[0,-1mm,1mm],[0,-1mm,2mm]])
path = bezier_curve(points: [[0,0,0],[0.5mm,0,0],[1mm,1mm,0]])
show progressive_sweep(profile,path,
 orientation: "rmf",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 3,max_sections: 33,max_deviation: 0.1mm
).nurbs_patches_tessellate(segments: 4)
