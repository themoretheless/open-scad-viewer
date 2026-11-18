//! Bind original radial carriers to their authored quotient rectangles.
use super::*;
use std::collections::BTreeMap;

#[cfg(test)]
pub(crate) struct RadialIncidence {
    pub periodic_axes: [usize; 2],
    pub rims: Vec<usize>,
}

pub(crate) struct RadialBand {
    pub face: usize,
    pub axis: usize,
    pub rims: [BTreeMap<usize, bool>; 2],
}

fn read_band(model: &Model, face: usize, remaining: &mut usize) -> Result<Option<RadialBand>> {
    let f = &model.faces[face];
    let s = &f.surface;
    if s.periodic_u == s.periodic_v || !f.holes.is_empty() {
        return Ok(None);
    }
    let axis = usize::from(s.periodic_v);
    let bounds = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let coedges = &model.loops[f.outer].coedges;
    let mut traces = Vec::new();
    for coedge in coedges {
        if coedge
            .trim()
            .is_some_and(|trim| trim.curve_parameter != model.edges[coedge.edge].curve.domain())
        {
            return Ok(None);
        }
        let trace = ChartTrace::read(coedge, true, remaining)?;
        charge(remaining, trace.poles().count())?;
        traces.push(trace);
    }
    let sides = full::rectangle_sides(coedges, bounds, &traces)?;
    full::verify_shores(&sides, axis)?;
    let [a, b] = [2 * (1 - axis), 2 * (1 - axis) + 1];
    if sides[a].keys().any(|edge| sides[b].contains_key(edge))
        || [a, b].into_iter().any(|rim| {
            sides[rim]
                .keys()
                .any(|edge| sides[2 * axis].contains_key(edge))
        })
    {
        return Ok(None);
    }
    Ok(Some(RadialBand {
        face,
        axis,
        rims: [sides[a].clone(), sides[b].clone()],
    }))
}

/// Prerequisites are exact original world lifts and simple, positively wound
/// trim loops. The result follows increasing meridian parameters across the
/// actual rim ownership and visits every face exactly once.
pub(crate) fn inspect_cycle(
    model: &Model,
    faces: &[usize],
    remaining: &mut usize,
) -> Result<Option<Vec<RadialBand>>> {
    if !(2..=256).contains(&faces.len()) {
        return Ok(None);
    }
    let mut bands = Vec::new();
    for &face in faces {
        let Some(band) = read_band(model, face, remaining)? else {
            return Ok(None);
        };
        bands.push(band);
    }
    order_cycle(bands, remaining)
}

fn order_cycle(bands: Vec<RadialBand>, remaining: &mut usize) -> Result<Option<Vec<RadialBand>>> {
    let mut starts = BTreeMap::new();
    let mut faces = std::collections::BTreeSet::new();
    for (i, band) in bands.iter().enumerate() {
        charge(remaining, band.rims[0].len() + band.rims[1].len())?;
        let signature = band.rims[0].keys().copied().collect::<Vec<_>>();
        if !faces.insert(band.face) || starts.insert(signature, i).is_some() {
            return Ok(None);
        }
    }
    let mut order = Vec::new();
    let mut current = 0;
    let mut visited = vec![false; bands.len()];
    for _ in 0..bands.len() {
        if std::mem::replace(&mut visited[current], true) {
            return Ok(None);
        }
        order.push(current);
        let rim = &bands[current].rims[1];
        let signature = rim.keys().copied().collect::<Vec<_>>();
        let Some(&next) = starts.get(&signature) else {
            return Ok(None);
        };
        if rim
            .iter()
            .any(|(edge, rev)| bands[next].rims[0].get(edge) != Some(&!*rev))
        {
            return Ok(None);
        }
        current = next;
    }
    if current != 0 {
        return Ok(None);
    }
    let mut bands = bands.into_iter().map(Some).collect::<Vec<_>>();
    Ok(Some(
        order
            .into_iter()
            .map(|i| bands[i].take().unwrap())
            .collect(),
    ))
}

#[cfg(test)]
pub(crate) fn inspect(
    model: &Model,
    faces: [usize; 2],
    remaining: &mut usize,
) -> Result<Option<RadialIncidence>> {
    let Some(bands) = inspect_cycle(model, &faces, remaining)? else {
        return Ok(None);
    };
    Ok(Some(RadialIncidence {
        periodic_axes: [bands[0].axis, bands[1].axis],
        rims: bands[0]
            .rims
            .iter()
            .flat_map(|rim| rim.keys().copied())
            .collect(),
    }))
}

pub(crate) struct RadialCycleContacts {
    pub shared: Vec<crate::boundary_hull_contact::Certificate>,
    pub disjoint: Vec<[usize; 2]>,
}

pub(crate) fn certify_cycle_contacts(
    model: &Model,
    faces: &[usize],
    remaining: &mut usize,
    pairs: &mut usize,
    max_pairs: usize,
) -> Result<Option<RadialCycleContacts>> {
    let Some(bands) = inspect_cycle(model, faces, remaining)? else {
        return Ok(None);
    };
    let surfaces = bands
        .iter()
        .map(|band| &model.faces[band.face].surface)
        .collect::<Vec<_>>();
    let axes = bands.iter().map(|band| band.axis).collect::<Vec<_>>();
    for height in 0..3 {
        if *remaining == 0 || *pairs == max_pairs {
            break;
        }
        let proof = nurbs_core::surface_injectivity::certify_radial_band_cycle(
            &surfaces,
            &axes,
            height,
            (*remaining).min(100000),
            (max_pairs - *pairs).min(100000),
        )?;
        charge(remaining, proof.cells)?;
        *pairs += proof.pairs;
        if proof.proven {
            return Ok(Some(cycle_contacts(model, &bands, remaining)?));
        }
    }
    Ok(None)
}

fn cycle_contacts(
    model: &Model,
    bands: &[RadialBand],
    remaining: &mut usize,
) -> Result<RadialCycleContacts> {
    let mut contacts = RadialCycleContacts {
        shared: Vec::new(),
        disjoint: Vec::new(),
    };
    for (i, a) in bands.iter().enumerate() {
        for (j, b) in bands.iter().enumerate().skip(i + 1) {
            charge(remaining, 1)?;
            let faces = [a.face.min(b.face), a.face.max(b.face)];
            let forward = j == i + 1;
            let wrap = i == 0 && j + 1 == bands.len();
            if !forward && !wrap {
                contacts.disjoint.push(faces);
                continue;
            }
            let rims = [wrap.then_some(&a.rims[0]), forward.then_some(&a.rims[1])];
            let edges = rims
                .into_iter()
                .flatten()
                .flat_map(|rim| rim.keys().copied())
                .collect::<Vec<_>>();
            contacts
                .shared
                .push(rim_certificate(model, faces, edges, remaining)?);
        }
    }
    Ok(contacts)
}

fn rim_certificate(
    model: &Model,
    faces: [usize; 2],
    edges: Vec<usize>,
    remaining: &mut usize,
) -> Result<crate::boundary_hull_contact::Certificate> {
    let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for &edge in &edges {
        let poles = &model.edges[edge].curve.control_points;
        charge(remaining, poles.len())?;
        for pole in poles {
            for (interval, &value) in enclosure.iter_mut().zip(pole) {
                interval[0] = interval[0].min(value);
                interval[1] = interval[1].max(value);
            }
        }
    }
    Ok(crate::boundary_hull_contact::Certificate {
        faces,
        contact_enclosure: enclosure,
        edges,
        vertex: None,
        joined_proof: None,
        polar_join: None,
        source_join: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band(face: usize, start: usize, end: usize) -> RadialBand {
        RadialBand {
            face,
            axis: face % 2,
            rims: [
                BTreeMap::from([(start, false)]),
                BTreeMap::from([(end, true)]),
            ],
        }
    }

    #[test]
    fn rim_ownership_orders_shuffled_bands_and_rejects_bad_cycles() {
        let fixture = || vec![band(2, 2, 3), band(0, 0, 1), band(3, 3, 0), band(1, 1, 2)];
        let ordered = order_cycle(fixture(), &mut 8).unwrap().unwrap();
        assert_eq!(
            ordered.iter().map(|b| b.face).collect::<Vec<_>>(),
            [2, 3, 0, 1]
        );
        assert!(order_cycle(fixture(), &mut 7).is_err());
        let mut wrong_direction = fixture();
        wrong_direction[0].rims[1].insert(3, false);
        assert!(order_cycle(wrong_direction, &mut 8).unwrap().is_none());
        let disconnected = vec![band(0, 0, 1), band(1, 1, 0), band(2, 2, 3), band(3, 3, 2)];
        assert!(order_cycle(disconnected, &mut 8).unwrap().is_none());
    }
}
