// @rush/1
// Fixed original initial frame; retained-patch proof includes the copied seam.
// Open profile surface: retained-patch error proof is not a native body proof.
// Solid must refuse conversion without native B-rep ownership.
profile = bezier_curve(points: [[4mm,0,0],[4mm,0,0.01mm]])
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 4mm)
show progressive_sweep(profile,path,
 orientation: "fixed",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],spacing: "arc_length",length_tolerance: 0.0001mm,length_max_cells: 100000,initial_sections: 33,max_sections: 129,max_deviation: 1mm
).nurbs_patches_tessellate(segments: 4)
