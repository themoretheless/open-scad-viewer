// @rush/1
// Original 17 stations and 0.05mm bound, with explicit profile/cap/station repair.
outer = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.05mm)
hole = circle_curve(center: [0,0,0],normal: [-1,0,0],radius: 0.025mm)
path = bezier_curve(points: [[0,0,0],[0.5mm,0,0],[1mm,1mm,0]])
show brep_progressive_sweep([[outer],[hole]],path,
 orientation: "fixed",spacing: "parameter",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 17,max_sections: 17,max_deviation: 0.05mm,length_tolerance: 0.0001mm,length_max_cells: 100000,
 circle_correction_tolerance: 0.000000001mm,circle_correction_quantum: 0.0000000000009094947017729282mm,circle_correction_max_work: 1000000,
 cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000
).brep_smooth_sweep_stations(wall_tolerance: 0.01mm,quantum: 0.0000000000009094947017729282mm,max_work: 1000000,smoothness_max_work: 2000000).brep_tessellate(segments: 2)
