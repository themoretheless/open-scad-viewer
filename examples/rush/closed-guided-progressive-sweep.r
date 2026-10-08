// @rush/1
// Closed guide supplies the frame; retained-patch proof includes the copied seam.
// Open profile surface: retained-patch error proof is not a native body proof.
// Solid must refuse conversion without native B-rep ownership.
profile = bezier_curve(points: [[5mm,0,1mm],[5mm,0,2mm]])
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 5mm)
rail = circle_curve(center: [0,0,1mm],normal: [0,0,1],radius: 5mm)
show progressive_sweep(profile,path,
 orientation_guide: rail,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 33,max_sections: 129,max_deviation: 2mm
).nurbs_patches_tessellate(segments: 4)
