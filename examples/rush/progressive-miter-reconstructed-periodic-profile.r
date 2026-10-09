// @rush/1
// Periodic polynomial profile with native bounded station reconstruction; actual G2 and Solid are separately audited.
outer = nurbs_curve(degree: 2, knots: [0,1,2,3,4,5,6,7,8],
 control_points: [[1mm,0,0],[0,1mm,0],[-1mm,0,0],[0,-1mm,0],[1mm,0,0],[0,1mm,0]],
 weights: [1,1,1,1,1,1], periodic: true)
show brep_progressive_miter_sweep([[outer]],
 points: [[0,0,0],[0,0,10mm]], normal: [1,0,0],
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,0deg], weights: [1,1]},
 center_law: {degree: 1, knots: [7,7,8,9,9], values: [[0mm,0mm,0mm],[1mm,0mm,0mm],[0mm,0mm,0mm]], weights: [1,1,1]},
 retained_wall_max_injectivity_cells: 100000,
 initial_steps: 2, max_steps: 2, max_deviation: 1mm
).brep_smooth_miter_stations(wall_tolerance: 0.5mm, quantum: 0.125mm, max_work: 10000, max_deviation: 2mm)
 .brep_tessellate(4)
