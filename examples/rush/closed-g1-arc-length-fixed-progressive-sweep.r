// @rush/1
// Original seam positions/directions agree; endpoint parameter speeds differ.
// Retained error is separate from retained G1/G2 and Solid ownership.
profile = bezier_curve(points: [[4mm,0,0],[4mm,0,0.01mm]])
path = nurbs_curve(degree: 2,knots: [0,0,0,0.25,0.25,0.5,0.5,0.75,0.75,1,1,1],control_points: [[4,0,0],[4,4,0],[0,4,0],[-4,4,0],[-4,0,0],[-4,-4,0],[0,-4,0],[4,-4,0],[4,0,0]],weights: [1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1,1.4142135623730951,1],periodic: false)
show progressive_sweep(profile,path,orientation: "fixed",normal: [0,0,1],spacing: "arc_length",length_tolerance: 0.0001mm,length_max_cells: 100000,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 33,max_sections: 33,max_deviation: 1mm
).nurbs_patches_tessellate(segments: 4)
