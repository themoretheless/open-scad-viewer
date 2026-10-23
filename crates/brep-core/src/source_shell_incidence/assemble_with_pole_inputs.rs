use super::*;

pub fn assemble_with_pole_inputs(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[AffineMaps],
    candidates: &[RootCandidates],
    poles: &[Pole],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    if faces.len() < 2
        || faces.len() > 4096
        || max_work == 0
        || max_work > 100_000_000
        || max_driver_cells > 100000
    {
        return Err(error("Choose bounded faces and exact shell work"));
    }
    let mut offsets = Vec::new();
    let mut total = 0;
    for face in faces {
        if face.is_empty() || face.len() > 16 {
            return Err(error("Face needs bounded original wires"));
        }
        let surface = face[0].edges()[0].surface();
        let mut starts = Vec::new();
        for wire in face {
            if wire.edges().iter().any(|e| e.surface() != surface) {
                return Err(error("Face wires must share the original surface"));
            }
            starts.push(total);
            total += wire.edges().len();
            if total > 65536 {
                return Err(error("Source shell incidence work limit"));
            }
        }
        offsets.push(starts);
    }
    if pairs
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(poles.len()))
        != Some(total)
    {
        return Err(error("Every directed source edge requires one partner"));
    }
    let mut supplied = BTreeMap::new();
    for spec in planes {
        if spec.pair >= pairs.len()
            || supplied.insert(spec.pair, spec.planes).is_some()
            || spec
                .planes
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err(error(
                "Plane inputs need unique valid pair addresses and finite anchors",
            ));
        }
    }
    let mut mapped = BTreeMap::new();
    for spec in maps {
        if spec.pair >= pairs.len()
            || mapped.insert(spec.pair, spec.ranges).is_some()
            || spec
                .ranges
                .iter()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || pairs[spec.pair].cutters.iter().any(Option::is_some)
        {
            return Err(error(
                "Affine maps need unique valid pair addresses, finite fractions and no cutter hints",
            ));
        }
    }
    let mut witnessed = BTreeMap::new();
    for spec in candidates {
        if spec.pair >= pairs.len()
            || witnessed.insert(spec.pair, spec.candidates).is_some()
            || spec
                .candidates
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || pairs[spec.pair].cutters.iter().any(Option::is_some)
        {
            return Err(error(
                "Root witnesses need unique valid addresses, finite original parameters and no cutter hints",
            ));
        }
    }
    let lookup = |a: Address| -> Result<usize> {
        let wire = faces
            .get(a.face)
            .and_then(|f| f.get(a.wire))
            .ok_or_else(|| error("Unknown source wire address"))?;
        if a.edge >= wire.edges().len() {
            return Err(error("Unknown source edge address"));
        }
        Ok(offsets[a.face][a.wire] + a.edge)
    };
    let mut seen = vec![false; total];
    for p in pairs {
        if p.uses[0].face == p.uses[1].face {
            return Err(error("Pair must join distinct faces"));
        }
        for &a in &p.uses {
            let i = lookup(a)?;
            if seen[i] {
                return Err(error("Source edge use paired more than once"));
            }
            seen[i] = true;
        }
    }
    for pole in poles {
        let i = lookup(pole.use_)?;
        if seen[i] {
            return Err(error("Source pole use claimed more than once"));
        }
        seen[i] = true;
    }
    let mut out = Report {
        shell: None,
        work_used: 0,
        root_checks: 0,
        driver_cells: 0,
        uncertain_pair: None,
        reason: "source-shell-pair-unqualified",
    };
    let mut parent: Vec<_> = (0..total).collect();
    let mut face_parent: Vec<_> = (0..faces.len()).collect();
    let mut edges = Vec::new();
    let mut qualified_poles = Vec::new();
    for pole in poles {
        if out.work_used == max_work {
            out.reason = "source-shell-pole-work-limit";
            return Ok(out);
        }
        let a = pole.use_;
        let source = &faces[a.face][a.wire].edges()[a.edge];
        let report = crate::source_collapsed_boundary::qualify(
            source,
            pole.point,
            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
        )?;
        out.work_used += report.work_used;
        let Some(proof) = report.boundary else {
            out.reason = report.reason;
            return Ok(out);
        };
        let vertices = faces[a.face][a.wire].vertices()[a.edge];
        join(
            &mut parent,
            offsets[a.face][a.wire] + vertices[0],
            offsets[a.face][a.wire] + vertices[1],
        );
        qualified_poles.push((a, proof));
    }
    for (i, p) in pairs.iter().enumerate() {
        if out.work_used == max_work {
            out.uncertain_pair = Some(i);
            out.reason = "source-shell-work-limit";
            return Ok(out);
        }
        let sources = p.uses.map(|a| &faces[a.face][a.wire].edges()[a.edge]);
        let report = if mapped.contains_key(&i) || witnessed.contains_key(&i) {
            let full = p.world_reversed.map(|r| {
                if r {
                    [[1., 1.], [0., 1.]]
                } else {
                    [[0., 1.], [1., 1.]]
                }
            });
            let ranges = mapped.get(&i).copied().unwrap_or(full);
            crate::source_mapped_edge::qualify_with_candidates(
                &p.world,
                sources,
                ranges,
                supplied.get(&i).copied().unwrap_or([None, None]),
                witnessed.get(&i).copied().unwrap_or([[None; 2]; 2]),
                max_work - out.work_used,
                max_driver_cells - out.driver_cells,
            )?
        } else {
            source_shared_edge::qualify_with_cutters_and_planes(
                &p.world,
                sources,
                p.world_reversed,
                [p.cutters[0].as_ref(), p.cutters[1].as_ref()],
                supplied.get(&i).copied().unwrap_or([None, None]),
                max_work - out.work_used,
                max_driver_cells - out.driver_cells,
            )?
        };
        out.work_used += report.work_used;
        out.root_checks += report.root_checks;
        out.driver_cells += report.driver_cells;
        let Some(edge) = report.edge else {
            out.uncertain_pair = Some(i);
            out.reason = report.reason;
            return Ok(out);
        };
        let endpoint = |a: Address, end: usize| {
            offsets[a.face][a.wire] + faces[a.face][a.wire].vertices()[a.edge][end]
        };
        join(&mut parent, endpoint(p.uses[0], 0), endpoint(p.uses[1], 1));
        join(&mut parent, endpoint(p.uses[0], 1), endpoint(p.uses[1], 0));
        join(&mut face_parent, p.uses[0].face, p.uses[1].face);
        edges.push(edge);
    }
    let first = root(&mut face_parent, 0);
    if (1..faces.len()).any(|i| root(&mut face_parent, i) != first) {
        out.reason = "source-shell-disconnected";
        return Ok(out);
    }
    let mut ids = BTreeMap::new();
    let vertices = faces
        .iter()
        .enumerate()
        .map(|(f, face)| {
            face.iter()
                .enumerate()
                .map(|(w, wire)| {
                    wire.vertices()
                        .iter()
                        .map(|v| {
                            v.map(|i| {
                                let owner = root(&mut parent, offsets[f][w] + i);
                                let next = ids.len();
                                *ids.entry(owner).or_insert(next)
                            })
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    out.shell = Some(Shell {
        poles: qualified_poles,
        faces: faces.to_vec(),
        regions: None,
        edges,
        uses: pairs.iter().map(|p| p.uses).collect(),
        vertices,
    });
    out.reason = "source-closed-oriented-incidence-qualified";
    Ok(out)
}
