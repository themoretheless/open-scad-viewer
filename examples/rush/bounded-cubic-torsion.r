// @rush/1
source = bezier_curve(points:[[0,0,0],[1mm,0,0],[2mm,1mm,0],[3mm,3mm,3mm]])
field0 = source.bounded_curve_torsion_field(piece:0,parameter:0.5,return_field:false,field_error:0.000001,max_work:1000000)
show field0.surface_extrude(vector:[0,0,1mm]).tessellate(16,4)
