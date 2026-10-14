// @rush/1
// Moving rational multispan authored frame; boundary and Solid proofs are separate.
profile = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 0.5mm)
path = line_curve(start: [0,0,0],end: [0,0,10mm])
show brep_progressive_sweep([[profile]],path,
 orientation: "authored",normal: [1,0,0],
 frame_axis: {degree: 3,knots: [0,0,0,0,0.25,0.5,0.75,1,1,1,1],
 values: [[0,0,1],[0.125,0,1],[0.375,0.125,1],[0,0.25,1],[-0.375,0.125,1],[-0.125,0,1],[0,0,1]],
 weights: [1,1,1,2,1,1,1]},
 frame_normal: {degree: 1,knots: [0,0,1,1],values: [[1,0,0],[1,0,0]],weights: [1,1]},
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 5,max_sections: 17,max_deviation: 2mm
).brep_tessellate(4)
