use super::*;

fn parameter_errors(proof: &QualifiedFiberWorldTrace) -> Result<[f64; 2]> {
    let f = proof.source();
    let op = f
        .report()
        .operator
        .as_ref()
        .ok_or_else(|| crate::input("Missing physical curve root operator"))?;
    let mut errors = [I::point(0.); 2];
    let mut unknown = 0;
    for k in 0..4 {
        if k != f.free_axis().index() {
            errors[k / 2] = errors[k / 2].add(
                I::point(op.scales()[unknown]).mul(I::point(proof.scaled_parameter_error()))?,
            )?;
            unknown += 1;
        }
    }
    Ok(errors.map(|i| i.hi))
}

pub(super) fn errors(t: &QualifiedPhysicalFiberTrace) -> Result<(f64, [f64; 2])> {
    let mut world = 0_f64;
    let mut uv = [0_f64; 2];
    for s in &t.segments {
        let tube = t
            .tubes
            .get(s.arc)
            .ok_or_else(|| crate::input("Missing physical arc tube"))?;
        world = world.max(
            I::point(s.proof.world_error_upper())
                .add(I::point(tube.world))?
                .hi,
        );
        let e = parameter_errors(&s.proof)?;
        for k in 0..2 {
            uv[k] = uv[k].max(I::point(e[k]).add(I::point(tube.uv[k]))?.hi);
        }
    }
    Ok((world, uv))
}

fn endpoint(c: &Curve, last: bool) -> &[f64] {
    if last {
        c.control_points.last().unwrap()
    } else {
        &c.control_points[0]
    }
}

fn charge(work: &mut Work, cells: usize) -> Result<()> {
    work.used = work
        .used
        .checked_add(cells)
        .filter(|&used| used <= work.max)
        .ok_or_else(|| crate::resource("Physical saved trace work exhausted"))?;
    Ok(())
}

pub(super) fn verify(
    t: &QualifiedPhysicalFiberTrace,
    fresh: bool,
    max_cells: usize,
) -> Result<usize> {
    validate_limits(t.limits)?;
    check(
        (1..=t.limits.max_cells).contains(&max_cells),
        "Invalid physical saved trace replay allowance",
    )?;
    if fresh {
        t.owner.reverify()?;
    }
    let b = t
        .owner
        .branches()
        .get(t.branch)
        .ok_or_else(|| crate::input("Unknown saved physical trace branch"))?;
    let atlas = t.owner.owner().owner();
    let axis = b.free_axis.index();
    let plan = plan(&t.owner, t.branch)?;
    let n = plan.len();
    check(
        n > 0
            && t.arcs.len() == n
            && t.ports.len() == n + 1
            && t.tubes.len() == n
            && !t.segments.is_empty()
            && t.segments.len() <= t.limits.max_segments
            && t.cells <= t.limits.max_cells
            && t.evaluations <= t.limits.max_evaluations,
        "Invalid physical saved trace shape or work",
    )?;
    let mut work = Work {
        used: 0,
        max: max_cells,
    };
    for i in 0..=n {
        let p = &t.ports[i];
        let expected_chart = if i == 0 { plan[0].0 } else { plan[i - 1].0 };
        let original = &atlas.fibers()[expected_chart];
        let expected_clock = if i == 0 {
            b.domain[0]
        } else {
            plan[i - 1].1[1]
        };
        check(
            same_source(&p.selector, original)
                && p.selector.free_axis() == b.free_axis
                && p.selector.domain()[axis] == [expected_clock; 2]
                && contained(p.selector.domain(), original.domain())
                && p.bounds == point_bounds(&p.selector)?
                && contained(p.bounds, original.domain())
                && p.parameters == p.bounds.map(midpoint),
            "Physical saved port selector or canonical parameters changed",
        )?;
        if i > 0 && i < n {
            let next = &atlas.fibers()[plan[i].0];
            check(
                contained(p.selector.domain(), next.domain()) && contained(p.bounds, next.domain()),
                "Physical saved interior port is not shared by both charts",
            )?;
        }
        if i == 0 || i == n {
            let end = if i == 0 { 0 } else { 1 };
            let vertex = &t.owner.vertices()[b.vertices[end]];
            check(
                vertex.chart() == expected_chart
                    && same_source(&p.selector, vertex.point())
                    && contained(p.selector.domain(), vertex.point().domain()),
                "Physical saved boundary port lost its original vertex",
            )?;
        }
        if fresh {
            charge(&mut work, p.selector.report().rectangles)?;
            p.selector.reverify()?;
            let world = world_bounds(p.selector.source().first(), p.bounds, &mut work)?;
            check(
                p.world == world.map(midpoint),
                "Physical saved port world enclosure changed",
            )?;
        }
    }

    let mut cursor = 0;
    for i in 0..n {
        let (chart, interval) = plan[i];
        let original = &atlas.fibers()[chart];
        let arc = &t.arcs[i];
        let f = &arc.fiber;
        check(
            arc.chart == chart
                && same_source(f, original)
                && f.free_axis() == b.free_axis
                && f.domain()[axis] == interval
                && contained(f.domain(), original.domain()),
            "Physical saved arc chart or clipping changed",
        )?;
        if fresh {
            charge(&mut work, f.report().rectangles)?;
            f.reverify()?;
        }
        let a = &t.ports[i];
        let b = &t.ports[i + 1];
        let tubes = &t.tubes[i];
        for (j, port) in [i, i + 1].into_iter().enumerate() {
            let band = &tubes.fibers[j];
            if fresh {
                charge(&mut work, band.report().rectangles)?;
                band.reverify()?;
            }
            check(
                same_source(band, f)
                    && band.free_axis() == f.free_axis()
                    && contained(band.domain(), f.domain())
                    && contained(t.ports[port].bounds, band.domain())
                    && band.domain()[axis][0] <= t.ports[port].parameters[axis]
                    && t.ports[port].parameters[axis] <= band.domain()[axis][1],
                "Physical endpoint tube source or root changed",
            )?;
            point_bounds(band)?;
        }
        check(
            tubes.fibers[0].domain()[axis][1] < tubes.fibers[1].domain()[axis][0],
            "Physical endpoint rephasing bands overlap",
        )?;
        if fresh {
            let (world, uv) = tube_errors(&tubes.fibers, &[a, b], &mut work)?;
            check(
                world == tubes.world && uv == tubes.uv,
                "Physical endpoint tube errors changed",
            )?;
        }
        check(
            tubes.world <= t.limits.tolerance_world / 4.
                && tubes.uv.iter().all(|x| *x <= t.limits.tolerance_uv / 4.),
            "Physical endpoint tube exceeds tolerance",
        )?;
        let first = cursor;
        let mut clock = a.parameters[axis];
        while cursor < t.segments.len() && t.segments[cursor].arc == i {
            let p = &t.segments[cursor].proof;
            let source = p.source();
            check(
                same_source(source, f)
                    && source.free_axis() == f.free_axis()
                    && contained(source.domain(), f.domain()),
                "Physical saved curve source changed",
            )?;
            let d = source.domain()[axis];
            check(
                d[0] == clock && p.orientation() == TransitionOrientation::Forward,
                "Physical native clocks do not tile",
            )?;
            clock = d[1];
            if fresh {
                charge(
                    &mut work,
                    p.rectangles()
                        .checked_add(p.agreement_cells())
                        .ok_or_else(|| crate::resource("Physical saved trace work exhausted"))?,
                )?;
                p.reverify()?;
            }
            let previous = if cursor == first {
                None
            } else {
                Some(&t.segments[cursor - 1].proof)
            };
            for k in 0..3 {
                let c = if k == 0 {
                    p.curve()
                } else {
                    &p.pcurves()[k - 1]
                };
                let expected = if let Some(prev) = previous {
                    endpoint(
                        if k == 0 {
                            prev.curve()
                        } else {
                            &prev.pcurves()[k - 1]
                        },
                        true,
                    )
                } else if k == 0 {
                    &a.world[..]
                } else {
                    &a.parameters[2 * (k - 1)..2 * k]
                };
                check(
                    endpoint(c, false) == expected,
                    "Saved physical curve endpoints disagree",
                )?;
            }
            cursor += 1;
        }
        check(
            cursor > first && clock == b.parameters[axis],
            "Physical arc has missing saved coverage",
        )?;
        let last = &t.segments[cursor - 1].proof;
        check(
            endpoint(last.curve(), true) == b.world
                && endpoint(&last.pcurves()[0], true) == &b.parameters[..2]
                && endpoint(&last.pcurves()[1], true) == &b.parameters[2..],
            "Saved physical arc does not meet its shared port",
        )?;
    }
    check(
        cursor == t.segments.len(),
        "Unordered extra physical trace segments",
    )?;
    let (world, uv) = errors(t)?;
    check(
        world == t.world_error
            && uv == t.uv_errors
            && world <= t.limits.tolerance_world
            && uv.iter().all(|v| *v <= t.limits.tolerance_uv),
        "Physical saved trace error exceeds tolerance",
    )?;
    Ok(work.used)
}
