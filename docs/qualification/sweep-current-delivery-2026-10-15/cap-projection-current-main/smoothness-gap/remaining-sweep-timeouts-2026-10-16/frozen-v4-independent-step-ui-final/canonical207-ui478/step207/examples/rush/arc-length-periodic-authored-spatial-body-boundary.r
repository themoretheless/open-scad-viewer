// @rush/1
// Periodic spatial source with a separately authored original derivative axis.
// Native retained-error qualification; global Solid and UI are separate gates.
outer = circle_curve(center: [-1.336111111111111mm,3.1694444444444443mm,-0.3388888888888889mm],normal: [-20.133333333333333,-7.866666666666666,-4.266666666666667],radius: 0.01mm)
path = nurbs_curve(degree: 6,knots: [-0.75,-0.625,-0.5,-0.375,-0.25,-0.125,0.0,0.125,0.25,0.375,0.5,0.625,0.75,0.875,1.0,1.125,1.25,1.375,1.5,1.625,1.75],control_points: [[4mm,0mm,0mm],[3mm,3mm,1mm],[0mm,4mm,0mm],[-3mm,3mm,-1mm],[-4mm,0mm,0mm],[-3mm,-3mm,1mm],[0mm,-4mm,0mm],[3mm,-3mm,-1mm],[4mm,0mm,0mm],[3mm,3mm,1mm],[0mm,4mm,0mm],[-3mm,3mm,-1mm],[-4mm,0mm,0mm],[-3mm,-3mm,1mm]],weights: [1,1,1,1,1,1,1,1,1,1,1,1,1,1],periodic: true)
show brep_progressive_sweep([[outer]],path,
 orientation: "authored",normal: [0,0,1],spacing: "arc_length",length_tolerance: 0.0001mm,length_max_cells: 100000,
 frame_axis: {degree: 5,knots: [-0.625, -0.5, -0.375, -0.25, -0.125, 0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0, 1.125, 1.25, 1.375, 1.5, 1.625],values: [[-8, 24, 8], [-24, 8, -8], [-24, -8, -8], [-8, -24, 8], [8, -24, 8], [24, -8, -8], [24, 8, -8], [8, 24, 8], [-8, 24, 8], [-24, 8, -8], [-24, -8, -8], [-8, -24, 8], [8, -24, 8]],weights: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]},
 frame_normal: {degree: 1,knots: [0,0,1,1],values: [[0,0,1],[0,0,1]],weights: [1,1]},
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 33,max_sections: 129,max_deviation: 0.5mm
).brep_tessellate(4)
