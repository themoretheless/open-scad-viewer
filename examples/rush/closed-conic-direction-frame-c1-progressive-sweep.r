// @rush/1
// Exact source direction C1; second direction jets and retained joins remain separate.
profile = bezier_curve(points: [[1mm,0,0.1mm],[1mm,0,0.2mm]])
path = nurbs_curve(degree: 2,knots: [2,2,2,2.75,2.75,3.5,3.5,4.25,4.25,5,5,5],control_points: [[1mm,0,0],[1mm,1mm,0],[0,1mm,0],[-1mm,1mm,0],[-1mm,0,0],[-1mm,-1mm,0],[0,-1mm,0],[1mm,-1mm,0],[1mm,0,0]],weights: [1,0.5,1,0.5,1,0.5,1,0.5,1])
show progressive_sweep(profile,path,
 orientation: "rmf",normal: [0,0,1],spacing: "arc_length",length_tolerance: 0.001mm,length_max_cells: 100000,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 5,max_sections: 65,max_deviation: 2mm
).nurbs_patches_tessellate(segments: 8)
