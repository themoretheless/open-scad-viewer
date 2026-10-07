// @rush/1
// Native certificate requires constructor-owned zero RMF seam correction.
profile = bezier_curve(points: [[5mm,0,-1mm],[5mm,0,1mm]])
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 5mm)
show progressive_sweep(profile,path,
 orientation: "rmf",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,360deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 5,max_sections: 257,max_deviation: 0.01mm
).nurbs_patches_tessellate(segments: 4)
