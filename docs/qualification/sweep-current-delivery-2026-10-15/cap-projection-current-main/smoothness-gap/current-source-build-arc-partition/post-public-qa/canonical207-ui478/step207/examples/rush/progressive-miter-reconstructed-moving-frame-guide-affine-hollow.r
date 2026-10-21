// @rush/1
// The authored axis controls longitudinal orientation; the guide controls the projected transverse normal.
outer = circle_curve(center: [0,0,0], normal: [0,0,1], radius: 0.5mm)
hole = circle_curve(center: [0,0,0], normal: [0,0,-1], radius: 0.2mm)
rail = line_curve(start: [1mm,0mm,0mm],end: [1mm,0mm,10mm])
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,0deg], weights: [1,1]},
 frame_axis: {degree: 1, knots: [2,2,5,5], values: [[0,0,2],[0,2,2]], weights: [1,1]},
 frame_normal: {degree: 1, knots: [7,7,9,9], values: [[3,0,0],[3,0,0]], weights: [1,1]},
 orientation_guide: rail,
 axis_scale: {degree: 1, knots: [17,17,19,19], values: [[2,1,1],[2,1,1]], weights: [1,1]},
 center_law: {degree: 1, knots: [23,23,29,29], values: [[0mm,0mm,0mm],[0mm,0mm,0mm]], weights: [1,1]},
 circle_correction_tolerance:1e-9mm,circle_correction_max_work:100000,cap_correction_tolerance: 0.6mm, cap_correction_quantum: 0.0000000000009094947017729282mm, cap_correction_max_work: 1000000,
 retained_wall_max_injectivity_cells: 10000,
 initial_steps: 1, max_steps: 16, max_deviation: 0.6mm
).brep_smooth_miter_stations(wall_tolerance:1mm,quantum:0.0000000000004547473508864641mm,max_work:1000000,max_deviation:2mm).brep_tessellate(4)
