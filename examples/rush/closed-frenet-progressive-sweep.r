// @rush/1
// Original rational circular path with moving tangent and curvature-derived Frenet frame.
// Retained-patch proof includes radial transport and the copied closed seam.
profile = bezier_curve(points: [[6mm,0,1mm],[6mm,0,2mm]])
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 5mm)
show progressive_sweep(profile,path,
 orientation: "frenet",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 33,max_sections: 129,max_deviation: 2mm
).nurbs_patches_tessellate(segments: 4)
