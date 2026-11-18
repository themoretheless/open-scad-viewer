//! Display cells on original material. No mesh replaces exact trim recipes.
use crate::{source_contour_proposal::SourceRegion, source_contour_winding};
use nurbs_core::{Error, Result, trim_domain::Location};
#[derive(Debug, PartialEq)]
pub struct Tile {
    pub uv: [[f64; 2]; 2],
    pub corners: [[f64; 3]; 4],
}
#[derive(Debug, PartialEq)]
pub struct Preview {
    pub tiles: Vec<Tile>,
    pub unresolved: Vec<[[f64; 2]; 2]>,
    pub outside: usize,
    pub domain_cells: usize,
}
/// Only whole rectangles proven material are evaluated for display. Boundary
/// bands remain explicit unresolved rectangles. Linear display triangles are
/// not a chord-error certificate or a closed tessellated body.
pub fn prepare(
    region: &SourceRegion,
    divisions: usize,
    tolerance_uv: f64,
    max_domain_cells: usize,
) -> Result<Preview> {
    if !(1..=64).contains(&divisions)
        || !(1..=100000).contains(&max_domain_cells)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(Error::new(
            "BREP_SOURCE_DISPLAY_LIMIT",
            "Bound display divisions and domain work",
        ));
    }
    let surface = region.loops()[0][0].surface();
    let u = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let v = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let coordinate = |d: [f64; 2], i: usize| {
        if i == divisions {
            d[1]
        } else {
            d[0] + (d[1] - d[0]) * (i as f64 / divisions as f64)
        }
    };
    let mut out = Preview {
        tiles: vec![],
        unresolved: vec![],
        outside: 0,
        domain_cells: 0,
    };
    for i in 0..divisions {
        for j in 0..divisions {
            let uv = [
                [coordinate(u, i), coordinate(u, i + 1)],
                [coordinate(v, j), coordinate(v, j + 1)],
            ];
            let material = if region.whole_chart_material() {
                true
            } else {
                if out.domain_cells == max_domain_cells {
                    out.unresolved.push(uv);
                    continue;
                }
                let report = source_contour_winding::classify(
                    region.loops(),
                    uv,
                    tolerance_uv,
                    max_domain_cells - out.domain_cells,
                )?;
                out.domain_cells += report.cells;
                if report.location == Location::Outside {
                    out.outside += 1;
                    continue;
                }
                report.location == Location::Inside
                    && report.winding == Some(region.chart_winding())
            };
            if !material {
                out.unresolved.push(uv);
                continue;
            }
            let mut corners = [[0.; 3]; 4];
            for (k, [a, b]) in [
                [uv[0][0], uv[1][0]],
                [uv[0][1], uv[1][0]],
                [uv[0][1], uv[1][1]],
                [uv[0][0], uv[1][1]],
            ]
            .into_iter()
            .enumerate()
            {
                corners[k] = surface.evaluate(a, b)?.point;
            }
            out.tiles.push(Tile { uv, corners });
        }
    }
    Ok(out)
}
