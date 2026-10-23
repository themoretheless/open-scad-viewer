use super::*;

/// `cells` counts edge direction enclosures, the initial normal, and each
/// transported corner. A numeric/limit ambiguity is unresolved, never accepted.
pub fn certify(
    points: &[[f64; 3]],
    normal: [f64; 3],
    closed: bool,
    miter_limit: f64,
    max_cells: usize,
) -> Result<Report> {
    check(
        if closed {
            (3..=16).contains(&points.len())
        } else {
            (2..=17).contains(&points.len())
        },
        "Frame certificate needs2..17 open or3..16 closed sites",
    )?;
    check(
        points
            .iter()
            .flatten()
            .chain(normal.iter())
            .all(|x| x.is_finite()),
        "Frame certificate inputs must be finite",
    )?;
    check(
        miter_limit.is_finite() && miter_limit >= 1. && max_cells <= 100000,
        "Invalid frame certificate limit/budget",
    )?;
    let mut cells = 0;
    let mut tick = || -> Result<()> {
        if cells == max_cells {
            return Err(crate::resource("Frame certificate cell budget exhausted"));
        }
        cells += 1;
        Ok(())
    };
    let result = (|| -> Result<(Vec<V>, Vec<V>, Vec<Option<V>>, Option<V>, Option<[f64; 2]>, Vec<I>)> {
        let edges = points.len() - usize::from(!closed);
        let mut tangents = Vec::new();
        let mut lengths = vec![I::point(0.)];
        for i in 0..edges {
            tick()?;
            let delta = sub(point(points[(i + 1) % points.len()]), point(points[i]))?;
            lengths.push(lengths[i].add(length(delta)?)?);
            tangents.push(unit(delta)?);
        }
        tick()?;
        let authored = unit(point(normal))?;
        let initial = unit(sub(
            authored,
            mul(tangents[0], dot(authored, tangents[0])?)?,
        )?)?;
        let mut normals = vec![initial];
        let mut planes = vec![None; points.len()];
        for i in 1..edges {
            tick()?;
            let (plane, n) = corner(tangents[i - 1], tangents[i], normals[i - 1], miter_limit)?;
            planes[i] = Some(plane);
            normals.push(n);
        }
        let closing = if closed {
            tick()?;
            let (plane, n) = corner(
                tangents[edges - 1],
                tangents[0],
                normals[edges - 1],
                miter_limit,
            )?;
            planes[0] = Some(plane);
            Some(n)
        } else {
            None
        };
        let angle = if let Some(end) = closing {
            tick()?;
            let sine = dot(tangents[0], cross(end, initial)?)?;
            let cosine = dot(end, initial)?;
            Some(
                super::super::trigonometric_certificate::certify_atan2(
                    [sine.lo, sine.hi],
                    [cosine.lo, cosine.hi],
                )?
                .ok_or_else(|| crate::resource("Holonomy angle enclosure unresolved"))?,
            )
        } else {
            None
        };
        Ok((tangents, normals, planes, closing, angle, lengths))
    })();
    match result {
        Ok((tangents, normals, planes, closing, angle, lengths)) => Ok(Report {
            status: Status::Certified,
            cells,
            tangents: Some(tangents.into_iter().map(encode).collect()),
            normals: Some(normals.into_iter().map(encode).collect()),
            planes: Some(planes.into_iter().map(|p| p.map(encode)).collect()),
            closing_normal: closing.map(encode),
            holonomy_angle: angle,
            distribute_holonomy:true,
            cumulative_lengths: Some(lengths.into_iter().map(|v| [v.lo, v.hi]).collect()),
            reason: None,
            sites: Some(points.to_vec()),
        }),
        Err(error) => Ok(unresolved(
            cells,
            if error.code == crate::RESOURCE_LIMIT && cells == max_cells {
                "cell-budget-exhausted"
            } else {
                "transport-enclosure-unresolved"
            },
        )),
    }
}
