// @rush/1
// Closed guide supplies the frame; retained-patch proof includes the copied seam.
// Qualification refusal case: the available continuous bound exceeds 100mm.
// Rush must refuse construction despite small sampled error.
profile = bezier_curve(points: [[5mm,0,1mm],[5mm,0,2mm]])
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 5mm)
rail = circle_curve(center: [0,0,1mm],normal: [0,0,1],radius: 5mm)
show progressive_sweep(profile,path,
 orientation_guide: rail,contact_profile: 0,contact_parameter: 0,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 33,max_sections: 33,max_deviation: 100mm
).nurbs_patches_tessellate(segments: 4)
