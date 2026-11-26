// @rush/1
a = line_curve(start:[0mm,0mm,0mm],end:[5mm,0mm,0mm])
b = line_curve(start:[4mm,3mm,0mm],end:[4mm,6mm,0mm])
c = line_curve(start:[3mm,3.5mm,0.5mm],end:[5mm,3.5mm,0.5mm])
result = a.routed_tube_bundle(inputs:[b,c],radii_mm:[0.5,1,0.25],tolerance_mm:0.000001,max_cells:20000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
