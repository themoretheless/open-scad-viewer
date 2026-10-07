// @rush/1
// Original rotating rational frame; retained boundary and material are audited separately.
profile = circle_curve(center: [4mm,0,0],normal: [0,1,0],radius: 0.2mm)
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 4mm)
show brep_progressive_sweep([[profile]],path,
 orientation: "authored",normal: [0,0,1],
 frame_axis: {degree: 2,knots: [0,0,0,0.25,0.25,0.5,0.5,0.75,0.75,1,1,1],
 values: [[0,1,0],[-1,1,0],[-1,0,0],[-1,-1,0],[0,-1,0],[1,-1,0],[1,0,0],[1,1,0],[0,1,0]],
 weights: [1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1]},
 frame_normal: {degree: 1,knots: [0,0,1,1],values: [[0,0,1],[0,0,1]],weights: [1,1]},
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 17,max_sections: 65,max_deviation: 2mm
).brep_tessellate(4)
