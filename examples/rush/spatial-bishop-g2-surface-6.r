// @rush/1
// A periodic surface, with independent Rust G2 and whole-surface proofs.
// Its open profile has two boundary rings; it is not a closed Solid shell.
p = bezier_curve(points:[[3.125mm,0,0],[3.25mm,0,0]])
c=nurbs_curve(degree:3,knots:[0, 0, 0, 0, 0.25, 0.25, 0.25, 0.5, 0.5, 0.5, 0.75, 0.75, 0.75, 1, 1, 1, 1],control_points:[[3mm,0mm,0mm],[3mm,1mm,0.125mm],[1mm,3mm,0.125mm],[0mm,3mm,0mm],[-1mm,3mm,-0.125mm],[-3mm,1mm,-0.125mm],[-3mm,0mm,0mm],[-3mm,-1mm,0.125mm],[-1mm,-3mm,0.125mm],[0mm,-3mm,0mm],[1mm,-3mm,-0.125mm],[3mm,-1mm,-0.125mm],[3mm,0mm,0mm]],weights:[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1])
show profile_sweep(p,c,scale:{degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]},normal:[1,0,0],sections:6,max_deviation:0.5mm).tessellate(4,64)
