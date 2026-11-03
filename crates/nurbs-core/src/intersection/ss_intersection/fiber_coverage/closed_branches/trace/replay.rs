use super::*;
pub(super) fn errors(t: &QualifiedClosedFiberTrace) -> Result<(f64, [f64; 2])> {
    let mut world = 0_f64;
    let mut uv = [0_f64; 2];
    for s in &t.segments {
        let tube = t
            .tubes
            .get(s.arc)
            .ok_or_else(|| crate::input("Missing closed arc tube"))?;
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
pub(super) fn verify(t: &QualifiedClosedFiberTrace, fresh: bool) -> Result<()> {
    validate_limits(t.limits)?;
    if fresh {
        t.cycle.reverify()?;
    }
    let n = t.cycle.arcs().len();
    check(
        n > 0
            && t.ports.len() == n
            && t.tubes.len() == n
            && !t.segments.is_empty()
            && t.segments.len() <= t.limits.max_segments
            && t.cells <= t.limits.max_cells
            && t.evaluations <= t.limits.max_evaluations,
        "Invalid closed saved trace shape or work",
    )?;
    let mut work = Work {
        used: 0,
        max: t.limits.max_cells,
    };
    let mut cursor = 0;
    for i in 0..n {
        let arc = &t.cycle.arcs()[i];
        let f = arc.fiber();
        let axis = f.free_axis().index();
        let a = &t.ports[i];
        let b = &t.ports[(i + 1) % n];
        check(
            a.parameters == t.cycle.ports()[i].parameter_bounds().map(midpoint),
            "Closed port parameters changed",
        )?;
        if fresh {
            let bounds = world_bounds(
                f.source().first(),
                t.cycle.ports()[i].parameter_bounds(),
                &mut work,
            )?;
            check(
                a.world == bounds.map(midpoint),
                "Closed port world enclosure changed",
            )?;
        }
        let tubes = &t.tubes[i];
        for (j, port) in [i, (i + 1) % n].into_iter().enumerate() {
            let band = &tubes.fibers[j];
            if fresh {
                band.reverify()?;
                work.used = work.used.saturating_add(band.report().rectangles);
                work.remaining()?;
            }
            check(
                same_source(band, f)
                    && band.free_axis() == f.free_axis()
                    && contained(band.domain(), f.domain())
                    && contained(t.cycle.ports()[port].parameter_bounds(), band.domain())
                    && band.domain()[axis][0] <= t.ports[port].parameters[axis]
                    && t.ports[port].parameters[axis] <= band.domain()[axis][1],
                "Closed endpoint tube source or root changed",
            )?;
            super::super::cycle::point_bounds(band)?;
        }
        let x = tubes.fibers[0].domain()[axis];
        let y = tubes.fibers[1].domain()[axis];
        check(
            if arc.forward() {
                x[1] < y[0]
            } else {
                y[1] < x[0]
            },
            "Closed endpoint rephasing bands overlap",
        )?;
        if fresh {
            let (world, uv) = tube_errors(&tubes.fibers, &[a, b], &mut work)?;
            check(
                world == tubes.world && uv == tubes.uv,
                "Closed endpoint tube errors changed",
            )?;
        }
        check(
            tubes.world <= t.limits.tolerance_world / 4.
                && tubes.uv.iter().all(|x| *x <= t.limits.tolerance_uv / 4.),
            "Closed endpoint tube exceeds tolerance",
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
                "Closed curve source changed",
            )?;
            let d = source.domain()[axis];
            let (start, end) = if arc.forward() {
                (d[0], d[1])
            } else {
                (d[1], d[0])
            };
            check(
                start == clock
                    && p.orientation()
                        == if arc.forward() {
                            TransitionOrientation::Forward
                        } else {
                            TransitionOrientation::Reverse
                        },
                "Closed native clocks do not tile",
            )?;
            clock = end;
            if fresh {
                p.reverify()?;
                work.used = work
                    .used
                    .saturating_add(p.rectangles() + p.agreement_cells());
                work.remaining()?;
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
                    "Saved closed curve endpoints disagree",
                )?;
            }
            cursor += 1;
        }
        check(
            cursor > first && clock == b.parameters[axis],
            "Closed arc has missing saved coverage",
        )?;
        let last = &t.segments[cursor - 1].proof;
        check(
            endpoint(last.curve(), true) == b.world
                && endpoint(&last.pcurves()[0], true) == &b.parameters[..2]
                && endpoint(&last.pcurves()[1], true) == &b.parameters[2..],
            "Closed saved arc does not meet its shared port",
        )?;
    }
    check(
        cursor == t.segments.len(),
        "Unordered extra closed trace segments",
    )?;
    let (world, uv) = errors(t)?;
    check(
        world == t.world_error
            && uv == t.uv_errors
            && world <= t.limits.tolerance_world
            && uv.iter().all(|v| *v <= t.limits.tolerance_uv),
        "Closed saved trace error exceeds tolerance",
    )?;
    Ok(())
}
