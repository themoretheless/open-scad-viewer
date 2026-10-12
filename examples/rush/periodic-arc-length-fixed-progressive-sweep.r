// @rush/1
// Exact original periodic seam and arc-length retained surface bound.
// Open profile: Solid must refuse without native B-rep ownership.
profile = bezier_curve(points: [[0,0,0],[0.01mm,0,0]])
path = nurbs_curve(degree: 6,knots: [-0.75,-0.625,-0.5,-0.375,-0.25,-0.125,0.0,0.125,0.25,0.375,0.5,0.625,0.75,0.875,1.0,1.125,1.25,1.375,1.5,1.625,1.75],control_points: [[4mm,0mm,0mm],[3mm,3mm,1mm],[0mm,4mm,0mm],[-3mm,3mm,-1mm],[-4mm,0mm,0mm],[-3mm,-3mm,1mm],[0mm,-4mm,0mm],[3mm,-3mm,-1mm],[4mm,0mm,0mm],[3mm,3mm,1mm],[0mm,4mm,0mm],[-3mm,3mm,-1mm],[-4mm,0mm,0mm],[-3mm,-3mm,1mm]],weights: [1,1,1,1,1,1,1,1,1,1,1,1,1,1],periodic: true)
show progressive_sweep(profile,path,orientation: "fixed",normal: [1,0,0],spacing: "arc_length",length_tolerance: 0.0001mm,length_max_cells: 100000,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 33,max_sections: 129,max_deviation: 1mm).nurbs_patches_tessellate(segments: 4)
