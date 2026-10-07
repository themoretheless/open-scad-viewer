// @rush/1
profile = bezier_curve(points: [[1mm,0,0],[2mm,0,0]])
path = bezier_curve(points: [[0,0,0],[0,0,0.5mm],[0,1mm,1mm]])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,7.16197243913529deg],weights: [1,1]},
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[1,1,1],[2,1,1]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0,0,0],[0.125mm,0.25mm,0]],weights: [1,1]},
 orientation: "rmf",
 normal: [1,0,0],spacing: "arc_length",initial_sections: 3,max_sections: 17,
 length_tolerance: 0.001mm,length_max_cells: 100000,max_deviation: 2mm
).nurbs_patches_tessellate(segments: 8)
