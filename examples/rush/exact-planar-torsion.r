// @rush/1
source = bezier_curve(points:[[0,0,0],[1.5mm,0,0],[3mm,3mm,0]])
field0 = source.curve_torsion_field(piece:0,parameter:0.5,return_field:false,max_work:1000000)
show field0.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
