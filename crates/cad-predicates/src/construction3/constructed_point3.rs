use super::*;

#[derive(Clone)]
pub struct ConstructedPoint3 {
    node: Arc<Node3>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructionKind3 {
    LinePlane,
    CoplanarEndpoint(OverlapEndpoint3),
}
pub(super) struct Node3 {
    recipe: ConstructionKind3,
    inputs: [StoredPoint3; 5],
    context: ContextIdentity,
    enclosure: [[f64; 2]; 3],
    line_parameter: [f64; 2],
    plane_parameters: [[f64; 2]; 2],
    diameter: f64,
    depth: usize,
    node_count: usize,
}
#[derive(Clone)]
pub(super) enum StoredPoint3 {
    Authored([LeafRef; 3]),
    Constructed(ConstructedPoint3),
}
impl StoredPoint3 {
    pub(super) fn borrowed(&self) -> Point3Input<'_> {
        match self {
            Self::Authored(p) => Point3Input::Authored(*p),
            Self::Constructed(p) => Point3Input::Constructed(p),
        }
    }
    pub(super) fn owned(input: Point3Input<'_>) -> Self {
        match input {
            Point3Input::Authored(p) => Self::Authored(p),
            Point3Input::Constructed(p) => Self::Constructed(p.clone()),
        }
    }
}
impl std::fmt::Debug for ConstructedPoint3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstructedPoint3")
            .field("kind", &self.kind())
            .field("context", &self.context())
            .field("enclosure", &self.enclosure())
            .field("depth", &self.recipe_depth())
            .field("nodes", &self.recipe_node_count())
            .finish_non_exhaustive()
    }
}
impl ConstructedPoint3 {
    pub fn kind(&self) -> ConstructionKind3 {
        self.node.recipe
    }
    pub fn context(&self) -> ContextIdentity {
        self.node.context
    }
    /// Ordered inputs: line A/B followed by plane C/D/E.
    pub fn inputs(&self) -> [Point3Input<'_>; 5] {
        self.node.inputs.each_ref().map(StoredPoint3::borrowed)
    }
    pub fn enclosure(&self) -> [[f64; 2]; 3] {
        self.node.enclosure
    }
    /// t in A+t(B-A), not clamped to a finite segment.
    pub fn line_parameter_enclosure(&self) -> [f64; 2] {
        self.node.line_parameter
    }
    /// u/v in C+u(D-C)+v(E-C), not clamped to a triangle or trim.
    pub fn plane_parameter_enclosures(&self) -> [[f64; 2]; 2] {
        self.node.plane_parameters
    }
    pub fn diameter_bound(&self) -> f64 {
        self.node.diameter
    }
    pub fn recipe_depth(&self) -> usize {
        self.node.depth
    }
    pub fn recipe_node_count(&self) -> usize {
        self.node.node_count
    }
    pub(super) fn key(&self) -> *const Node3 {
        Arc::as_ptr(&self.node)
    }
}
#[derive(Clone, Copy)]
pub enum Point3Input<'a> {
    Authored([LeafRef; 3]),
    Constructed(&'a ConstructedPoint3),
}
#[derive(Clone, Debug)]
pub enum LinePlaneIntersection3 {
    Unique(Box<ConstructedPoint3>),
    Parallel,
    Contained,
    DegenerateLine,
    DegeneratePlane,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct Construction3Report {
    pub outcome: LinePlaneIntersection3,
    pub context: ContextIdentity,
    pub work_used: u64,
}
pub(super) fn values(ctx: &PredicateContext<'_>, p: [LeafRef; 3]) -> Result<[AuthoredScalar; 3], InputError> {
    Ok([
        ctx.resolve(p[0])?.clone(),
        ctx.resolve(p[1])?.clone(),
        ctx.resolve(p[2])?.clone(),
    ])
}
pub(super) struct Inventory {
    pub(super) nodes: usize,
    pub(super) depth: usize,
    pub(super) work: u64,
}
pub(super) fn inventory(
    ctx: &PredicateContext<'_>,
    inputs: &[Point3Input<'_>],
) -> Result<Inventory, InputError> {
    // At most twelve already-admitted roots, each with at most 256 nodes. Private
    // immutable constructors guarantee acyclicity before this bounded admission.
    let mut seen = HashSet::new();
    let mut pending = inputs.to_vec();
    let mut depth = 0;
    let mut work = 0;
    while let Some(input) = pending.pop() {
        work += 1;
        match input {
            Point3Input::Authored(p) => {
                values(ctx, p)?;
                work += 3;
            }
            Point3Input::Constructed(p) => {
                if p.context() != ctx.identity() {
                    return Err(InputError::ContextMismatch);
                }
                depth = depth.max(p.recipe_depth());
                if seen.insert(p.key()) {
                    pending.extend(p.inputs());
                }
            }
        }
    }
    Ok(Inventory {
        nodes: seen.len(),
        depth,
        work,
    })
}
pub(super) fn authored(value: &[AuthoredScalar; 3], ctx: &mut PredicateContext<'_>) -> Result<HPoint, Reason> {
    let [xn, xd] = fraction(&value[0], ctx)?;
    let [yn, yd] = fraction(&value[1], ctx)?;
    let [zn, zd] = fraction(&value[2], ctx)?;
    normalize(
        [
            xn.mul(&yd, ctx)?.mul(&zd, ctx)?,
            yn.mul(&xd, ctx)?.mul(&zd, ctx)?,
            zn.mul(&xd, ctx)?.mul(&yd, ctx)?,
            xd.mul(&yd, ctx)?.mul(&zd, ctx)?,
        ],
        ctx,
    )
}
pub(super) fn determinant3(
    a: [&Expansion; 3],
    b: [&Expansion; 3],
    c: [&Expansion; 3],
    ctx: &mut PredicateContext<'_>,
) -> Result<Expansion, Reason> {
    let x = b[1].mul(c[2], ctx)?.sub(&b[2].mul(c[1], ctx)?, ctx)?;
    let y = b[2].mul(c[0], ctx)?.sub(&b[0].mul(c[2], ctx)?, ctx)?;
    let z = b[0].mul(c[1], ctx)?.sub(&b[1].mul(c[0], ctx)?, ctx)?;
    a[0].mul(&x, ctx)?
        .add(&a[1].mul(&y, ctx)?, ctx)?
        .add(&a[2].mul(&z, ctx)?, ctx)
}
pub(super) fn plane(
    a: &HPoint,
    b: &HPoint,
    c: &HPoint,
    ctx: &mut PredicateContext<'_>,
) -> Result<HPoint, Reason> {
    // Signed 3x3 minors give coefficients n.x*X+n.y*Y+n.z*Z+d*W=0.
    let minor = |indices: [usize; 3], ctx: &mut PredicateContext<'_>| {
        determinant3(
            indices.map(|i| &a[i]),
            indices.map(|i| &b[i]),
            indices.map(|i| &c[i]),
            ctx,
        )
    };
    normalize(
        [
            minor([1, 2, 3], ctx)?,
            minor([0, 2, 3], ctx)?.negated(),
            minor([0, 1, 3], ctx)?,
            minor([0, 1, 2], ctx)?.negated(),
        ],
        ctx,
    )
}
pub(super) fn dot(a: &HPoint, b: &HPoint, ctx: &mut PredicateContext<'_>) -> Result<Expansion, Reason> {
    let mut result = a[0].mul(&b[0], ctx)?;
    for i in 1..4 {
        result = result.add(&a[i].mul(&b[i], ctx)?, ctx)?;
    }
    Ok(result)
}
pub(super) fn delta(b: &HPoint, a: &HPoint, ctx: &mut PredicateContext<'_>) -> Result<[Expansion; 3], Reason> {
    Ok([
        b[0].mul(&a[3], ctx)?.sub(&a[0].mul(&b[3], ctx)?, ctx)?,
        b[1].mul(&a[3], ctx)?.sub(&a[1].mul(&b[3], ctx)?, ctx)?,
        b[2].mul(&a[3], ctx)?.sub(&a[2].mul(&b[3], ctx)?, ctx)?,
    ])
}
pub(super) fn zero<const N: usize>(value: &[Expansion; N]) -> bool {
    value.iter().all(|x| x.sign() == Sign::Zero)
}
pub(super) enum ExactResult {
    Unique {
        point: HPoint,
        inputs: Box<[HPoint; 5]>,
    },
    Parallel,
    Contained,
    DegenerateLine,
    DegeneratePlane,
}
pub(super) fn solve(inputs: [HPoint; 5], ctx: &mut PredicateContext<'_>) -> Result<ExactResult, Reason> {
    let [a, b, c, d, e] = &inputs;
    let direction = delta(b, a, ctx)?;
    if zero(&direction) {
        return Ok(ExactResult::DegenerateLine);
    }
    let support = plane(c, d, e, ctx)?;
    if zero(&support) {
        return Ok(ExactResult::DegeneratePlane);
    }
    let ra = dot(&support, a, ctx)?;
    let rb = dot(&support, b, ctx)?;
    if ra.sign() == Sign::Zero && rb.sign() == Sign::Zero {
        return Ok(ExactResult::Contained);
    }
    let mut point = normalize(
        [
            rb.mul(&a[0], ctx)?.sub(&ra.mul(&b[0], ctx)?, ctx)?,
            rb.mul(&a[1], ctx)?.sub(&ra.mul(&b[1], ctx)?, ctx)?,
            rb.mul(&a[2], ctx)?.sub(&ra.mul(&b[2], ctx)?, ctx)?,
            rb.mul(&a[3], ctx)?.sub(&ra.mul(&b[3], ctx)?, ctx)?,
        ],
        ctx,
    )?;
    if point[3].sign() == Sign::Zero {
        return Ok(ExactResult::Parallel);
    }
    if point[3].sign() == Sign::Negative {
        point = point.map(|x| x.negated());
    }
    if dot(&support, &point, ctx)?.sign() != Sign::Zero {
        return Err(Reason::PrecisionExhausted);
    }
    let displacement = delta(&point, a, ctx)?;
    // Exact collinearity in every coordinate, including non-axis-aligned lines.
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        if displacement[i]
            .mul(&direction[j], ctx)?
            .sub(&displacement[j].mul(&direction[i], ctx)?, ctx)?
            .sign()
            != Sign::Zero
        {
            return Err(Reason::PrecisionExhausted);
        }
    }
    Ok(ExactResult::Unique {
        point,
        inputs: Box::new(inputs),
    })
}
pub(super) fn quotient(
    n: &Expansion,
    d: &Expansion,
    ctx: &mut PredicateContext<'_>,
) -> Result<Interval, Reason> {
    n.enclosure(ctx)?.div_nonzero(&d.enclosure(ctx)?, ctx)
}
pub(super) fn line_parameter_fraction(
    point: &HPoint,
    a: &HPoint,
    b: &HPoint,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; 2], Reason> {
    let direction = delta(b, a, ctx)?;
    let displacement = delta(point, a, ctx)?;
    for i in 0..3 {
        if direction[i].sign() != Sign::Zero {
            return Ok([
                displacement[i].mul(&b[3], ctx)?,
                direction[i].mul(&point[3], ctx)?,
            ]);
        }
    }
    Err(Reason::MissingProof)
}
pub(super) fn plane_parameter_fractions(
    point: &HPoint,
    c: &HPoint,
    d: &HPoint,
    e: &HPoint,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; 3], Reason> {
    let u = delta(d, c, ctx)?;
    let v = delta(e, c, ctx)?;
    let p = delta(point, c, ctx)?;
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let det = u[i].mul(&v[j], ctx)?.sub(&u[j].mul(&v[i], ctx)?, ctx)?;
        if det.sign() != Sign::Zero {
            let den = det.mul(&point[3], ctx)?;
            let un = p[i]
                .mul(&v[j], ctx)?
                .sub(&p[j].mul(&v[i], ctx)?, ctx)?
                .mul(&d[3], ctx)?;
            let vn = u[i]
                .mul(&p[j], ctx)?
                .sub(&u[j].mul(&p[i], ctx)?, ctx)?
                .mul(&e[3], ctx)?;
            return Ok([un, vn, den]);
        }
    }
    Err(Reason::MissingProof)
}
#[derive(Default)]
pub(super) struct Evaluation {
    cache: HashMap<*const Node3, HPoint>,
    terms: usize,
}
impl Evaluation {
    pub(super) fn point(
        &mut self,
        input: Point3Input<'_>,
        ctx: &mut PredicateContext<'_>,
    ) -> Result<HPoint, Reason> {
        ctx.charge(1)?;
        match input {
            Point3Input::Authored(p) => {
                authored(&values(ctx, p).map_err(|_| Reason::MissingProof)?, ctx)
            }
            Point3Input::Constructed(recipe) => {
                if let Some(point) = self.cache.get(&recipe.key()) {
                    ctx.charge(point.iter().map(Expansion::term_count).sum::<usize>() as u64)?;
                    return Ok(point.clone());
                }
                let [a, b, c, d, e] = recipe.inputs();
                let inputs = [
                    self.point(a, ctx)?,
                    self.point(b, ctx)?,
                    self.point(c, ctx)?,
                    self.point(d, ctx)?,
                    self.point(e, ctx)?,
                ];
                let point = match recipe.node.recipe {
                    ConstructionKind3::LinePlane => {
                        let ExactResult::Unique { point, .. } = solve(inputs, ctx)? else {
                            return Err(Reason::MissingProof);
                        };
                        point
                    }
                    ConstructionKind3::CoplanarEndpoint(endpoint) => {
                        clip_endpoint(&inputs, endpoint, ctx)?
                    }
                };
                let terms = point.iter().map(Expansion::term_count).sum::<usize>();
                ctx.charge(terms as u64)?;
                if self.terms + terms > MAX_CACHED_TERMS
                    || self.cache.len() >= MAX_CONSTRUCTION_NODES
                {
                    return Err(Reason::ResourceLimit);
                }
                self.terms += terms;
                self.cache.insert(recipe.key(), point.clone());
                Ok(point)
            }
        }
    }
}
pub(super) fn publish_point3(
    ctx: &mut PredicateContext<'_>,
    inputs: [Point3Input<'_>; 5],
    inventory: &Inventory,
    point: &HPoint,
    exact: &[HPoint; 5],
    recipe: ConstructionKind3,
) -> Result<ConstructedPoint3, Reason> {
    let bounds = [
        quotient(&point[0], &point[3], ctx)?,
        quotient(&point[1], &point[3], ctx)?,
        quotient(&point[2], &point[3], ctx)?,
    ];
    let mut diameter = Interval::point(0.);
    for bound in bounds {
        diameter = diameter.add(&Interval::point(bound.width()), ctx)?;
    }
    let diameter = inputs.iter().fold(diameter.hi, |bound, input| match input {
        Point3Input::Authored(_) => bound,
        Point3Input::Constructed(p) => bound.max(p.diameter_bound()),
    });
    if !diameter.is_finite() || diameter > ctx.tolerance.spec.max_entity_error {
        return Err(Reason::PrecisionExhausted);
    }
    let [tn, td] = line_parameter_fraction(point, &exact[0], &exact[1], ctx)?;
    let t = quotient(&tn, &td, ctx)?;
    let [un, vn, den] = plane_parameter_fractions(point, &exact[2], &exact[3], &exact[4], ctx)?;
    let uv = [quotient(&un, &den, ctx)?, quotient(&vn, &den, ctx)?];
    Ok(ConstructedPoint3 {
        node: Arc::new(Node3 {
            recipe,
            inputs: inputs.map(StoredPoint3::owned),
            context: ctx.identity(),
            enclosure: bounds.map(|b| [b.lo, b.hi]),
            line_parameter: [t.lo, t.hi],
            plane_parameters: uv.map(|b| [b.lo, b.hi]),
            diameter,
            depth: inventory.depth + 1,
            node_count: inventory.nodes + 1,
        }),
    })
}
/// A/B define the infinite line; C/D/E define the oriented infinite plane.
pub fn intersect_line_plane3d(
    ctx: &mut PredicateContext<'_>,
    a: Point3Input<'_>,
    b: Point3Input<'_>,
    c: Point3Input<'_>,
    d: Point3Input<'_>,
    e: Point3Input<'_>,
) -> Result<Construction3Report, InputError> {
    let inputs = [a, b, c, d, e];
    let inventory = inventory(ctx, &inputs)?;
    let outcome = (|| {
        ctx.charge(inventory.work)?;
        if inventory.depth >= MAX_CONSTRUCTION_DEPTH || inventory.nodes >= MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        let mut evaluation = Evaluation::default();
        let exact = [
            evaluation.point(a, ctx)?,
            evaluation.point(b, ctx)?,
            evaluation.point(c, ctx)?,
            evaluation.point(d, ctx)?,
            evaluation.point(e, ctx)?,
        ];
        let result = match solve(exact, ctx)? {
            ExactResult::Parallel => LinePlaneIntersection3::Parallel,
            ExactResult::Contained => LinePlaneIntersection3::Contained,
            ExactResult::DegenerateLine => LinePlaneIntersection3::DegenerateLine,
            ExactResult::DegeneratePlane => LinePlaneIntersection3::DegeneratePlane,
            ExactResult::Unique {
                point,
                inputs: exact,
            } => LinePlaneIntersection3::Unique(Box::new(publish_point3(
                ctx,
                inputs,
                &inventory,
                &point,
                &exact,
                ConstructionKind3::LinePlane,
            )?)),
        };
        ctx.charge(0)?;
        Ok(result)
    })();
    Ok(Construction3Report {
        outcome: outcome.unwrap_or_else(LinePlaneIntersection3::Indeterminate),
        context: ctx.identity(),
        work_used: ctx.work_used(),
    })
}
fn interval_point(
    point: Point3Input<'_>,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Interval; 3], Reason> {
    match point {
        Point3Input::Authored(p) => {
            let v = values(ctx, p).map_err(|_| Reason::MissingProof)?;
            Ok([
                Interval::authored(&v[0], ctx)?,
                Interval::authored(&v[1], ctx)?,
                Interval::authored(&v[2], ctx)?,
            ])
        }
        Point3Input::Constructed(p) => {
            ctx.charge(3)?;
            Ok(p.enclosure().map(|[lo, hi]| Interval { lo, hi }))
        }
    }
}
/// Plane-side / tetrahedral orientation with exact recipe fallback.
pub fn orient3d_points(
    ctx: &mut PredicateContext<'_>,
    a: Point3Input<'_>,
    b: Point3Input<'_>,
    c: Point3Input<'_>,
    d: Point3Input<'_>,
) -> Result<Decision, InputError> {
    if let (
        Point3Input::Authored(a),
        Point3Input::Authored(b),
        Point3Input::Authored(c),
        Point3Input::Authored(d),
    ) = (a, b, c, d)
    {
        return crate::orient3d(ctx, a, b, c, d);
    }
    let inventory = inventory(ctx, &[a, b, c, d])?;
    let mut stage = Stage::Admission;
    let result = (|| {
        ctx.charge(inventory.work)?;
        if inventory.depth > MAX_CONSTRUCTION_DEPTH || inventory.nodes > MAX_CONSTRUCTION_NODES {
            return Err(Reason::ResourceLimit);
        }
        stage = Stage::Interval;
        let points = [
            interval_point(a, ctx)?,
            interval_point(b, ctx)?,
            interval_point(c, ctx)?,
            interval_point(d, ctx)?,
        ];
        let flattened: Vec<_> = points.into_iter().flatten().collect();
        if let Some(sign) = crate::expression(crate::Predicate::Orient3, &flattened, ctx)?.sign() {
            ctx.charge(0)?;
            return Ok(sign);
        }
        stage = Stage::ExactExpansion;
        let mut evaluation = Evaluation::default();
        let a = evaluation.point(a, ctx)?;
        let b = evaluation.point(b, ctx)?;
        let c = evaluation.point(c, ctx)?;
        let d = evaluation.point(d, ctx)?;
        let sign = dot(&plane(&a, &b, &c, ctx)?, &d, ctx)?.sign();
        ctx.charge(0)?;
        Ok(sign)
    })();
    Ok(Decision {
        outcome: match result {
            Ok(sign) => Outcome::Sign(sign),
            Err(reason) => Outcome::Indeterminate(reason),
        },
        stage,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

/// Exact algebraic location relative to the recipe's original finite segment.
/// This is independent of modelling tolerances and does not merge nearby points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentLocation3 {
    BeforeStart,
    Start,
    Interior,
    End,
    AfterEnd,
}
/// Original triangle vertices are C/D/E. Edge indices are opposite C/D/E.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriangleLocation3 {
    Outside,
    Interior,
    Edge(usize),
    Vertex(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FiniteLocation3 {
    pub segment: SegmentLocation3,
    pub triangle: TriangleLocation3,
}
impl FiniteLocation3 {
    /// Includes exact endpoints, edges and vertices of both closed domains.
    pub fn is_closed_domain_hit(self) -> bool {
        !matches!(
            self.segment,
            SegmentLocation3::BeforeStart | SegmentLocation3::AfterEnd
        ) && self.triangle != TriangleLocation3::Outside
    }
}
#[derive(Clone, Debug)]
pub struct FiniteLocation3Report {
    pub outcome: Result<FiniteLocation3, Reason>,
    pub context: ContextIdentity,
    pub work_used: u64,
}
