// @rush/1
// Original right-angle miter remains C0 after bounded wall reconstruction.
outer = circle_curve(center: [0,0,0], normal: [0,0,1], radius: 1mm)
show brep_progressive_miter_sweep([[outer]],
 points: [[0,0,0],[0,0,10mm],[10mm,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,0deg], weights: [1,1]},
 circle_correction_tolerance: 0.000000001mm, circle_correction_quantum: 0.125mm,
 circle_correction_max_work: 10000, retained_wall_max_injectivity_cells: 100000,
 initial_steps: 1, max_steps: 1, max_deviation: 0.1mm
).brep_smooth_miter_stations(wall_tolerance: 0.1mm, quantum: 0.03125mm, max_work: 10000, max_deviation: 0.5mm)
 .brep_tessellate(4)
