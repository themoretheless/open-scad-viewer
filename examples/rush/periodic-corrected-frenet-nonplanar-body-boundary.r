// @rush/1
// Closed nonplanar source with exact position/velocity/acceleration seam.
// Native retained-error qualification; global Solid and UI are separate gates.
outer = circle_curve(center: [-1.336111111111111mm,3.1694444444444443mm,-0.3388888888888889mm],normal: [-20.133333333333333,-7.866666666666666,-4.266666666666667],radius: 0.01mm)
path = nurbs_curve(degree: 6,knots: [-0.75,-0.625,-0.5,-0.375,-0.25,-0.125,0.0,0.125,0.25,0.375,0.5,0.625,0.75,0.875,1.0,1.125,1.25,1.375,1.5,1.625,1.75],control_points: [[4mm,0mm,0mm],[3mm,3mm,1mm],[0mm,4mm,0mm],[-3mm,3mm,-1mm],[-4mm,0mm,0mm],[-3mm,-3mm,1mm],[0mm,-4mm,0mm],[3mm,-3mm,-1mm],[4mm,0mm,0mm],[3mm,3mm,1mm],[0mm,4mm,0mm],[-3mm,3mm,-1mm],[-4mm,0mm,0mm],[-3mm,-3mm,1mm]],weights: [1,1,1,1,1,1,1,1,1,1,1,1,1,1],periodic: true)
show brep_progressive_sweep([[outer]],path,
 orientation: "corrected_frenet",normal: [0,0,1],spacing: "parameter",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 5,max_sections: 129,max_deviation: 0.05mm
).brep_tessellate(4)
