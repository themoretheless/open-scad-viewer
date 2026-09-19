use brep_core::{
    Body, Model, bicubic_open_face, cylinder, export_step_v5, export_step_v6, export_step_v8,
    export_step_v9, freeform_cuboid_solid, frustum, sphere, torus, tube,
};
use nurbs_core::surface::Surface;

fn main() {
    let kind = std::env::args().nth(1).expect("fixture kind");
    let v9 = kind.starts_with("v9-");
    let v8 = kind.starts_with("v8-");
    let v6 = kind.starts_with("v6-");
    let kind = kind
        .strip_prefix("v6-")
        .or_else(|| kind.strip_prefix("v8-"))
        .or_else(|| kind.strip_prefix("v9-"))
        .unwrap_or(&kind)
        .to_string();
    let base_kind = match kind.as_str() {
        "nested-placement" | "assembly" | "metre" | "inch" | "periodic-cylinder" => "cylinder",
        "periodic-cone" => "cone",
        "periodic-torus" => "torus",
        "pole-sphere" => "sphere",
        "pole-cone" => "pole-cone",
        other => other,
    };
    let model = if kind == "open-shell" {
        open_shell()
    } else if kind == "multiple-void" {
        multiple_void_model()
    } else {
        match base_kind {
            "cylinder" => cylinder(2., 3.),
            "cone" => frustum(3., 1., 4.),
            "torus" => torus(4., 1.),
            "sphere" => sphere(2.),
            "pole-cone" => frustum(2., 0., 3.),
            "void" => tube(5., 2., 6.),
            _ => panic!("unknown fixture kind"),
        }
        .expect("fixture model")
    };
    let mut text = if v9 {
        export_step_v9(&model).expect("STEP /9 export").0
    } else if v8 {
        export_step_v8(&model).expect("STEP /8 export").0
    } else if v6 {
        export_step_v6(&model).expect("STEP /6 export").0
    } else {
        export_step_v5(&model).expect("STEP /5 export").0
    };
    if kind == "metre" {
        text = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)")
    }
    if kind == "inch" {
        let unit = entity_id(&text, "SI_UNIT(");
        text = text.replace(
            &format!("GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))"),
            "GLOBAL_UNIT_ASSIGNED_CONTEXT((#60002))",
        );
        text = text.replacen(
            "ENDSEC;\nEND-ISO",
            "\
#60000=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));
#60001=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#60000);
#60002=(CONVERSION_BASED_UNIT('INCH',#60001)LENGTH_UNIT()NAMED_UNIT(*));
ENDSEC;\nEND-ISO",
            1,
        );
    }
    if kind == "nested-placement" {
        text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let root = entity_id(&text, "=ADVANCED_BREP_SHAPE_REPRESENTATION");
        let body = entity_id(&text, "=MANIFOLD_SOLID_BREP");
        let context = entity_id(&text, "GLOBAL_UNIT_ASSIGNED_CONTEXT");
        let rows=format!("\
#60000=CARTESIAN_POINT('',(10.,0.,0.));
#60001=CARTESIAN_POINT('',(0.,20.,0.));
#60002=DIRECTION('',(1.,0.,0.));
#60003=DIRECTION('',(0.,1.,0.));
#60004=DIRECTION('',(0.,0.,1.));
#60005=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,#60002,#60003,#60000,1.,#60004);
#60006=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,#60002,#60003,#60001,1.,#60004);
#60007=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#60005),#{context});
#60008=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body}),#{context});
#60009=(REPRESENTATION_RELATIONSHIP('','',#{root},#60007)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#60005)SHAPE_REPRESENTATION_RELATIONSHIP());
#60010=(REPRESENTATION_RELATIONSHIP('','',#60007,#60008)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#60006)SHAPE_REPRESENTATION_RELATIONSHIP());
");
        text = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
    }
    if kind == "assembly" {
        text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let root = entity_id(&text, "=ADVANCED_BREP_SHAPE_REPRESENTATION");
        let body = entity_id(&text, "=MANIFOLD_SOLID_BREP");
        let context = entity_id(&text, "GLOBAL_UNIT_ASSIGNED_CONTEXT");
        let definition = entity_id(&text, "=PRODUCT_DEFINITION(");
        let definition_line = text
            .lines()
            .find(|line| line.starts_with(&format!("#{definition}=")))
            .unwrap();
        let refs = refs_in_line(definition_line);
        let rows=format!("\
#62000=CARTESIAN_POINT('',(10.,0.,0.));
#62001=CARTESIAN_POINT('',(-10.,0.,0.));
#62002=DIRECTION('',(1.,0.,0.));
#62003=DIRECTION('',(0.,1.,0.));
#62004=DIRECTION('',(0.,0.,1.));
#62005=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,#62002,#62003,#62000,1.,#62004);
#62006=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,#62002,#62003,#62001,1.,#62004);
#62007=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body}),#{context});
#62008=(REPRESENTATION_RELATIONSHIP('occurrence-a','',#{root},#62007)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#62005)SHAPE_REPRESENTATION_RELATIONSHIP());
#62009=(REPRESENTATION_RELATIONSHIP('occurrence-b','',#{root},#62007)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#62006)SHAPE_REPRESENTATION_RELATIONSHIP());
#62010=PRODUCT_DEFINITION('shared-child','',#{},#{});
#62011=NEXT_ASSEMBLY_USAGE_OCCURRENCE('occurrence-a','child A','',#{definition},#62010,'A');
#62012=NEXT_ASSEMBLY_USAGE_OCCURRENCE('occurrence-b','child B','',#{definition},#62010,'B');
",refs[0],refs[1]);
        text = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
    }
    if let Some(path) = std::env::args().nth(2) {
        std::fs::write(path, text).expect("write fixture")
    } else {
        print!("{text}")
    }
}

fn open_shell() -> Model {
    let control_points = (0..4)
        .map(|u| {
            (0..4)
                .map(|v| vec![u as f64 / 3., v as f64 / 3., (u * v) as f64 / 90.])
                .collect()
        })
        .collect();
    bicubic_open_face(Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    })
    .unwrap()
}

fn entity_id(text: &str, marker: &str) -> usize {
    let line = text
        .lines()
        .find(|line| line.starts_with('#') && line.contains(marker))
        .expect("fixture entity");
    line[1..line.find('=').unwrap()].parse().unwrap()
}
fn refs_in_line(line: &str) -> Vec<usize> {
    line.split('#')
        .skip(2)
        .filter_map(|part| {
            part.split(|ch: char| !ch.is_ascii_digit())
                .next()?
                .parse()
                .ok()
        })
        .collect()
}

fn multiple_void_model() -> Model {
    let mut model = freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap();
    append(
        &mut model,
        freeform_cuboid_solid([1., 1., 1.], [3., 3., 3.]).unwrap(),
    );
    append(
        &mut model,
        freeform_cuboid_solid([6., 6., 6.], [8., 8., 8.]).unwrap(),
    );
    model.0.bodies = vec![Body {
        outer_shell: 0,
        inner_shells: vec![1, 2],
    }];
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
fn append(target: &mut Model, source: Model) {
    let Model(mut topology, _) = source;
    let (vo, eo, lo, fo, so) = (
        target.vertices.len(),
        target.edges.len(),
        target.loops.len(),
        target.faces.len(),
        target.shells.len(),
    );
    for edge in &mut topology.edges {
        edge.vertices = [edge.vertices[0] + vo, edge.vertices[1] + vo]
    }
    for loop_ in &mut topology.loops {
        for coedge in &mut loop_.coedges {
            coedge.edge += eo
        }
    }
    for face in &mut topology.faces {
        face.outer += lo;
        for hole in &mut face.holes {
            *hole += lo
        }
    }
    for shell in &mut topology.shells {
        for face in &mut shell.faces {
            face.face += fo
        }
    }
    for body in &mut topology.bodies {
        body.outer_shell += so;
        for shell in &mut body.inner_shells {
            *shell += so
        }
    }
    target.0.vertices.extend(topology.vertices);
    target.0.edges.extend(topology.edges);
    target.0.loops.extend(topology.loops);
    target.0.faces.extend(topology.faces);
    target.0.shells.extend(topology.shells);
    target.0.bodies.extend(topology.bodies);
}
