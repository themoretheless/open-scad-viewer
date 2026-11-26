// @rush/1
edge = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm])
panel = edge.surface_extrude([0mm,1mm,0mm])
contacts = panel.order_boundary_contacts(clocks:[{linear:"0",constant:"-2",upper_root:true},{rational:"3/2"},{rational:"7/5"},{linear:"0",constant:"-2",upper_root:true}],isolate:true,max_work:100000)
show contacts.tessellate(4,4)
