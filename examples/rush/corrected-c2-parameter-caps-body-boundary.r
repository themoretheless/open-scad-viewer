// @rush/1
// A regular cubic C2 source with a real interior knot; frame second jets remain separate.
outer = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.1mm)
hole = circle_curve(center: [0,0,0],normal: [-1,0,0],radius: 0.05mm)
path = nurbs_curve(degree: 3,knots: [0,0,0,0,0.5,1,1,1,1],control_points: [[0mm,0mm,0mm],[0.16666666666666666mm,0mm,0mm],[0.5mm,0.16666666666666666mm,0mm],[0.8333333333333334mm,0.6666666666666666mm,0.5mm],[1mm,1mm,1.125mm]],weights: [1,1,1,1,1])
show brep_progressive_sweep([[outer],[hole]],path,
 orientation: "corrected_frenet",normal: [0,0,1],spacing: "parameter",
 circle_correction_tolerance: 0.000000001mm,circle_correction_quantum: 0.0000000000009094947017729282mm,circle_correction_max_work: 1000000,
 cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 3,max_sections: 129,max_deviation: 0.05mm
).brep_tessellate(4)
