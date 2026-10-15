// @rush/1
// Original circular hollow source; native automatic caps, fresh Solid admission.
outer = circle_curve(center: [0,0,0],normal: [3,4,0],radius: 0.1mm)
hole = circle_curve(center: [0,0,0],normal: [-3,-4,0],radius: 0.05mm)
path = line_curve(start: [0,0,0],end: [3mm,4mm,0])
show brep_progressive_sweep([[outer],[hole]],path,
 orientation: "rmf",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,2],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 3,max_deviation: 0.01mm
).brep_smooth_sweep_stations(wall_tolerance: 0.000000001mm,quantum: 9.094947017729282e-13mm,max_work: 1000000,smoothness_max_work: 2000000).brep_tessellate(2)
