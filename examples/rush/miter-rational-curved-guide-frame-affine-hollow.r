// @rush/1
// The authored axis controls longitudinal orientation; the guide controls the projected transverse normal.
outer = circle_curve(center: [0,0,0], normal: [0,0,1], radius: 0.5mm)
hole = circle_curve(center: [0,0,0], normal: [0,0,-1], radius: 0.2mm)
rail = nurbs_curve(degree: 2,knots: [31,31,31,41,41,41],control_points: [[0mm,1mm,0mm],[0.1mm,1mm,5mm],[0mm,1mm,10mm]],weights: [1,0.8,1])
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,0deg], weights: [1,1]},
 frame_axis: {degree: 1, knots: [2,2,5,5], values: [[0,0,2],[0,0,2]], weights: [1,1]},
 frame_normal: {degree: 1, knots: [7,7,9,9], values: [[3,0,0],[3,0,0]], weights: [1,1]},
 orientation_guide: rail,
 axis_scale: {degree: 1, knots: [17,17,19,19], values: [[2,1,1],[2,1,1]], weights: [1,1]},
 center_law: {degree: 1, knots: [23,23,29,29], values: [[0.125mm,0mm,0.25mm],[0.125mm,0mm,0.25mm]], weights: [1,1]},
 circle_correction_tolerance: 0.000000001mm,circle_correction_max_work: 100000,retained_wall_max_injectivity_cells: 100000,
 initial_steps: 1, max_steps: 16, max_deviation: 0.05mm
).brep_tessellate(4)
