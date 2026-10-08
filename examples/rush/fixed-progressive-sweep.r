// @rush/1
// Fixed frame from the original initial tangent; scalar laws vary continuously.
// Retained surface error does not establish a native Solid body.
profile = bezier_curve(points: [[1mm,0,0],[2mm,0,0]])
path = line_curve(start: [0,0,0],end: [0,0,10mm])
show progressive_sweep(profile,path,
 orientation: "fixed",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,90deg],weights: [1,1]},
 normal: [1,0,0],initial_sections: 3,max_sections: 33,max_deviation: 0.05mm
).nurbs_patches_tessellate(segments: 4)
