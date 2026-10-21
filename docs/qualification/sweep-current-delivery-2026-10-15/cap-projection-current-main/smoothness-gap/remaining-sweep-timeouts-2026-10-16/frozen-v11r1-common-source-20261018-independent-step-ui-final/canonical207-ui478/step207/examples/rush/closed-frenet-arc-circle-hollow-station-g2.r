// @rush/1
// Exact original raw-WASM ABI request; no change to stations, tolerance or work budgets.
profile0 = nurbs_curve(degree: 2,knots: [0,0,0,0.25,0.25,0.5,0.5,0.75,0.75,1,1,1],control_points: [[5.01mm,0mm,0mm],[5.01mm,0mm,-0.01mm],[5mm,0mm,-0.01mm],[4.99mm,0mm,-0.01mm],[4.99mm,0mm,0mm],[4.99mm,0mm,0.01mm],[5mm,0mm,0.01mm],[5.01mm,0mm,0.01mm],[5.01mm,0mm,0mm]],weights: [1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1],periodic: false)
profile1 = nurbs_curve(degree: 2,knots: [0,0,0,0.25,0.25,0.5,0.5,0.75,0.75,1,1,1],control_points: [[5.005mm,0mm,0mm],[5.005mm,0mm,0.005mm],[5mm,0mm,0.005mm],[4.995mm,0mm,0.005mm],[4.995mm,0mm,0mm],[4.995mm,0mm,-0.005mm],[5mm,0mm,-0.005mm],[5.005mm,0mm,-0.005mm],[5.005mm,0mm,0mm]],weights: [1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1],periodic: false)
path = nurbs_curve(degree: 2,knots: [0,0,0,0.25,0.25,0.5,0.5,0.75,0.75,1,1,1],control_points: [[5mm,0mm,0mm],[5mm,5mm,0mm],[0mm,5mm,0mm],[-5mm,5mm,0mm],[-5mm,0mm,0mm],[-5mm,-5mm,0mm],[0mm,-5mm,0mm],[5mm,-5mm,0mm],[5mm,0mm,0mm]],weights: [1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1],periodic: false)
show brep_progressive_sweep([[profile0],[profile1]],path,
 orientation: "frenet",spacing: "arc_length",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 33,max_sections: 129,max_deviation: 2mm,
 length_tolerance: 0.0001mm,length_max_cells: 100000,
 circle_correction_quantum: 9.094947017729282e-13mm,circle_correction_tolerance: 1e-9mm,circle_correction_max_work: 1000000
).brep_smooth_sweep_stations(wall_tolerance: 0.01mm,quantum: 9.094947017729282e-13mm,max_work: 1000000,smoothness_max_work: 2000000).brep_tessellate(2)
