// @rush/1
// Closed nonplanar source with exact position/velocity/acceleration seam.
// Native retained-error qualification; global Solid and UI are separate gates.
outer = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.01mm)
path = bezier_curve(points: [[0,0,0],[1mm,0,0],[2mm,1mm,0],[3mm,3mm,1mm],[-3mm,3mm,1mm],[-2mm,1mm,0],[-1mm,0,0],[0,0,0]])
show brep_progressive_sweep([[outer]],path,
 orientation: "corrected_frenet",normal: [0,0,1],spacing: "parameter",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 axis_scale: {degree: 2,knots: [0,0,0,1,1,1],values: [[1.25,0.75,1.1],[1.5,0.5,1.2],[1.25,0.75,1.1]],weights: [1,1,1]},
 center_law: {degree: 2,knots: [0,0,0,1,1,1],values: [[0.001mm,-0.002mm,0.003mm],[0.002mm,-0.001mm,0.004mm],[0.001mm,-0.002mm,0.003mm]],weights: [1,1,1]},
 initial_sections: 5,max_sections: 129,max_deviation: 0.05mm
).brep_tessellate(4)
