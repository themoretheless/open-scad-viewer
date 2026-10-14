// @rush/1
// Original arc-length position and authored pose; surface and Solid are separate.
profile = bezier_curve(points: [[4.1mm,0,0],[4.2mm,0,0]])
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 4mm)
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 orientation: "authored",normal: [1,0,0],
 frame_axis: {degree: 1,knots: [0,0,1,1],values: [[0,0,1],[0,0,1]],weights: [1,1]},
 frame_normal: {degree: 1,knots: [0,0,1,1],values: [[1,0,0],[1,0,0]],weights: [1,1]},
 spacing: "arc_length",initial_sections: 5,max_sections: 33,
 length_tolerance: 0.001mm,length_max_cells: 100000,max_deviation: 2mm
).nurbs_patches_tessellate(segments: 8)
