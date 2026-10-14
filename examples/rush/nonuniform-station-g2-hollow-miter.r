// @rush/1
// Unequal retained station speeds require exact rational G2 reparameterization.
outer = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 0.5mm)
hole = circle_curve(center: [0,0,0],normal: [0,0,-1],radius: 0.25mm)
show brep_progressive_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,3mm],[0,0,10mm],[0,0,21mm],[0,0,34mm]],normal: [1,0,0],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 circle_correction_tolerance: 0.000000001mm,circle_correction_max_work: 100000,
 retained_wall_max_injectivity_cells: 100000,
 initial_steps: 1,max_steps: 2,max_deviation: 0.01mm
).brep_tessellate(4)
