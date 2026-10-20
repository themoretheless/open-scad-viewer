// @rush/1
// The authored axis controls longitudinal orientation; the guide controls the projected transverse normal.
outer = circle_curve(center: [0,0,0], normal: [0,0,1], radius: 0.5mm)
hole = circle_curve(center: [0,0,0], normal: [0,0,-1], radius: 0.2mm)
rail = line_curve(start: [1mm,0mm,0mm],end: [1mm,0mm,10mm])
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1.25], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,15deg], weights: [1,1]},
 orientation_guide: rail,
 cap_correction_tolerance: 2mm, cap_correction_quantum: 0.0000000000009094947017729282mm, cap_correction_max_work: 1000000,
 retained_wall_max_injectivity_cells: 10000,
 initial_steps: 1, max_steps: 16, max_deviation: 2mm
).brep_tessellate(4)
