// @rush/1
// Path and guide use independent normalized arc-length traversals.
profile = bezier_curve(points: [[1mm,0,0],[2mm,0,0]])
path = bezier_curve(points: [[0,0,0],[0,0,10mm]],weights: [1,2])
rail = bezier_curve(points: [[1mm,0,0],[2mm,1mm,10mm]],weights: [1,4])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 orientation_guide: rail,normal: [1,0,0],spacing: "arc_length",
 initial_sections: 3,max_sections: 9,length_tolerance: 0.001mm,length_max_cells: 100000,max_deviation: 0.05mm
).nurbs_patches_tessellate(segments: 8)
