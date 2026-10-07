// @rush/1
// Complete native boundary refinement continues after adding cap correction.
outer = circle_curve(center: [0,0,0], normal: [0,0,1], radius: 0.5mm)
hole = circle_curve(center: [0,0,0], normal: [0,0,-1], radius: 0.2mm)
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1.25], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,15deg], weights: [1,1]},
 frame_axis: {degree: 1, knots: [2,2,5,5], values: [[0,0,2],[0,2,2]], weights: [1,1]},
 frame_normal: {degree: 1, knots: [7,7,9,9], values: [[3,0,0],[3,0,0]], weights: [1,1]},
 axis_scale: {degree: 1, knots: [17,17,19,19], values: [[2,1,1],[2,1,1]], weights: [1,1]},
 center_law: {degree: 1, knots: [23,23,29,29], values: [[0.125mm,-0.25mm,0mm],[0.125mm,-0.25mm,0mm]], weights: [1,1]},
 cap_correction_tolerance: 2mm, cap_correction_quantum: 0.0000000000009094947017729282mm, cap_correction_max_work: 1000000,
 retained_wall_max_injectivity_cells: 10000,
 initial_steps: 1, max_steps: 16, max_deviation: 2mm
).brep_tessellate(4)
