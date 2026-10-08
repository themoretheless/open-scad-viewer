// @rush/1
// Native corrected circular hollow boundary; fresh Solid admission is required.
outer = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.1mm)
hole = circle_curve(center: [0,0,0],normal: [-1,0,0],radius: 0.05mm)
path = bezier_curve(points: [[0,0,0],[0.5mm,0,0],[1mm,1mm,0]])
show brep_progressive_sweep([[outer],[hole]],path,
 cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000,
 orientation: "rmf",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 129,max_deviation: 0.01mm
).brep_tessellate(4)
