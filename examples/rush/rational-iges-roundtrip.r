// @rush/1
body = brep_cylinder(radius:3mm,height:5mm)
file = body.export_iges_body()
restored = file.reopen_iges_body()
show restored.brep_tessellate(8)
