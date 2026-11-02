use modeling_informed::geometry::{
    boolean::{self, Boolean},
    math::Bounds,
    mesh::Mesh,
};
use modeling_informed::{Display, Document, Operation, Resolution};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args()
        .nth(1)
        .ok_or("usage: two_versions OUTPUT_DIRECTORY")?;
    let directory = std::path::Path::new(&destination);
    std::fs::create_dir_all(directory)?;
    let a = Bounds::new([0.; 3], [4.; 3])?;
    let b = Bounds::new([1., 1., 1.], [3., 3., 5.])?;
    // First version: stateless direct geometry computation.
    let clean = boolean::boxes(&[a], &[b], Boolean::Difference)?;
    // Second version: authoring document and background publication.
    let mut doc = Document::new()?;
    let left = doc.apply(Operation::Box { bounds: a })?.entities[0];
    let right = doc.apply(Operation::Box { bounds: b })?.entities[0];
    let job = doc.prepare(Operation::BoxBoolean {
        left,
        right,
        kind: Boolean::Difference,
    });
    let computed = std::thread::spawn(move || job.run())
        .join()
        .map_err(|_| "job panicked")??;
    let result = doc.publish(computed)?.entities[0];
    let display = doc.display(result, Resolution::default())?;
    let Display::Triangles(informed) = &*display else {
        return Err("mesh expected".into());
    };
    assert_eq!(&clean, informed);
    assert!(clean.topology().is_closed_manifold());
    for (name, mesh) in [("clean", &clean), ("informed", informed)] {
        std::fs::write(directory.join(format!("{name}.obj")), mesh.to_obj())?;
        std::fs::write(directory.join(format!("{name}.stl")), mesh.to_stl()?)?;
    }
    let round_profile = modeling_informed::geometry::profile::rounded_rectangle([6., 4.], 0.5, 12)?;
    let rounded = Mesh::extrude(&round_profile, 2.)?;
    std::fs::write(directory.join("rounded-profile.obj"), rounded.to_obj())?;
    println!(
        "Both versions: volume={}, triangles={}, closed={}; document revision={}",
        clean.signed_volume(),
        clean.triangles().len(),
        clean.topology().is_closed_manifold(),
        doc.revision()
    );
    doc.undo()?;
    assert!(doc.entity(result).is_none());
    doc.redo()?;
    assert!(doc.entity(result).is_some());
    Ok(())
}
