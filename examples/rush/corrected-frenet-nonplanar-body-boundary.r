// @rush/1
// Regular original spatial corrected frame; full body error and global admission are separately audited.
outer = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.1mm)
path = bezier_curve(points: [[0,0,0],[1mm,0,0],[2mm,1mm,0],[3mm,1mm,1mm]])
show brep_progressive_sweep([[outer]],path,
 orientation: "corrected_frenet",normal: [0,0,1],spacing: "parameter",
 cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 129,max_deviation: 0.05mm
).brep_tessellate(4)
