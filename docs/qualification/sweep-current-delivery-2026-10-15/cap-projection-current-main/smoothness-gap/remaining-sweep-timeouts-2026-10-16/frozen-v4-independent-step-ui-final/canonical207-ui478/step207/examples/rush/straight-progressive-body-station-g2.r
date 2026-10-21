// @rush/1
// Explicit station repair preserves the original complete boundary budget.
profile = circle_curve(center: [0,0,0], normal: [0,0,1], radius: 0.5mm)
path = line_curve(start: [0,0,0], end: [0,0,10mm])
show brep_progressive_sweep([[profile]],path,
 scale: {degree: 1, knots: [0,0,1,1], values: [1,1], weights: [1,1]},
 twist: {degree: 1, knots: [0,0,1,1], values: [0deg,0deg], weights: [1,1]},
 normal: [1,0,0], orientation: "rmf",initial_sections: 3,max_sections: 3,max_deviation: 0.01mm,
 circle_correction_tolerance: 0.000000001mm,circle_correction_quantum: 0.125mm,circle_correction_max_work: 1000000
).brep_smooth_sweep_stations(wall_tolerance: 0.000000001mm,quantum: 0.125mm,max_work: 1000000,smoothness_max_work: 2000000).brep_tessellate(2)
