// @rush/1
// Closed authored axes follow corner bisectors; the elevated rail controls transverse orientation.
// A conservative 10 mm bound admits the retained coarse family. Sharp corners remain C0.
outer = circle_curve(center: [0,0,0], normal: [1,0,0], radius: 0.25mm)
hole = circle_curve(center: [0,0,0], normal: [-1,0,0], radius: 0.1mm)
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[10mm,0,0],[10mm,10mm,0],[0,10mm,0]],normal: [0,0,1],closed: true,
 scale: {degree: 2,knots: [0,0,0,1,1,1],values: [1,1.25,1],weights: [1,2,1]},
 twist: {degree: 2,knots: [0,0,0,1,1,1],values: [0deg,15deg,0deg],weights: [1,2,1]},
 frame_axis: {degree: 1, knots: [0,0,0.25,0.5,0.75,1,1],values: [[1,-1,0],[1,1,0],[-1,1,0],[-1,-1,0],[1,-1,0]],weights: [1,1,1,1,1]},
 frame_normal: {degree: 1, knots: [2,2,5,5],values: [[0,0,1],[0,0,1]],weights: [1,1]},
 retained_wall_max_injectivity_cells: 10000,initial_steps: 1,max_steps: 8,max_deviation: 10mm
).brep_tessellate(4)
