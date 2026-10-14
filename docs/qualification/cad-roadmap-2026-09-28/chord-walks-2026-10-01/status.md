# Directed represented boundary walks

Added directed edge adjacency and clockwise successor traversal of the chord construction graph. Outgoing directions use half-plane ordering and outward interval orientation predicates; unresolved same-ray/near-collinear order is refused. Each directed edge participates in one walk. Area sign is enclosed after translating to the first cycle vertex; uncertain or zero area is refused.

The result distinguishes counterclockwise and clockwise walks by proved represented-coordinate area sign. These names do not claim global face membership: disconnected/nested cycles need region reconciliation and winding selection. Construction vertex uncertainties still need topology checks before attributing the result to the original offset.

Three native tests passed: square with either source direction; crossed bowtie yielding two triangular counterclockwise walks; duplicate rays and collapsed edges refused.

Open: containment, winding selection, contacts/overlap reconciliation, graph embedding admission, trimmed region reconstruction and application integration. No trimmed offset region is certified yet.
