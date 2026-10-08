// @rush/1
// Closed original authored frame C2 is independent of retained seams and Solid.
profile = bezier_curve(points: [[0.1mm,0,0],[0.2mm,0,0]])
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 4mm)
show progressive_sweep(profile,path,
 orientation: "authored",normal: [1,0,0],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 frame_axis: {degree: 5,knots: [0,0,0,0,0,0,1,1,1,1,1,1],
 values: [[0,0,1],[0.125,0,1],[0.25,0.125,1],[-0.25,0.125,1],[-0.125,0,1],[0,0,1]],
 weights: [1,1,1,1,1,1]},
 frame_normal: {degree: 1,knots: [0,0,1,1],values: [[1,0,0],[1,0,0]],weights: [1,1]},
 initial_sections: 5,max_sections: 65,max_deviation: 2mm
).nurbs_patches_tessellate(segments: 8)
