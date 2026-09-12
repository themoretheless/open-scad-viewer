//! Leaf math microbenchmark via rbench (use a release build).
//!
//! ```sh
//! cargo run --release -p math-core --example bench_leaf_math -- --profile quick
//! ```
use math_core::{add, cross, det, dot, mm, mv, rotation, scale, unit};
use rbench::{DropPolicy, Suite};

fn main() -> rbench::Result<()> {
    let mut suite = Suite::new("math-core");
    suite
        .bench_with_input(
            "leaf/hotpath",
            || {
                (
                    [1.1_f64, -2.2, 3.3],
                    [0.4_f64, 0.5, -0.6],
                    [[1.0_f64, 0.2, 0.0], [0.1, 1.0, 0.3], [0.0, 0.1, 1.0]],
                )
            },
            |(a, b, m)| {
                let c = cross(*a, *b);
                dot(unit(add(c, scale(*b, 0.1))), mv(*m, *a)) + det(mm(*m, rotation(*a)))
            },
            DropPolicy::InsideTiming,
        )
        .parameter("ops", 1);
    suite.main()
}
