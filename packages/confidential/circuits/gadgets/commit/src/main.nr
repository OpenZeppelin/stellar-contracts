use stellar_confidential_lib::commit;

// Minimal circuit wrapping `commit(v, r)`. Exists so `nargo info` can report
// the per-primitive constraint cost (the lib has no `main` of its own). Public
// inputs are the commitment coordinates; private witnesses are the opening.
fn main(v: Field, r: Field, cx: pub Field, cy: pub Field) {
    let c = commit(v, r);
    assert(c.x == cx);
    assert(c.y == cy);
}
