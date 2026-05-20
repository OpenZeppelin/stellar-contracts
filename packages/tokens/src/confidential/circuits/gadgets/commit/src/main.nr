use stellar_confidential_lib::commit;

// Minimal circuit wrapping `commit(v, r)`. Exists so `nargo info` can report
// the per-primitive constraint cost (the lib has no `main` of its own). Public
// inputs are the commitment coordinates; private witnesses are the opening.
fn main(v: Field, r: Field, cx: pub Field, cy: pub Field) {
    let c = commit(v, r);
    assert(c.x == cx);
    assert(c.y == cy);
}

#[test]
fn matches_fixture() {
    // Pinned from testdata/commit.json -- same vector as lib's fixtures_match_testdata.
    main(
        1000,
        42,
        0x195a5d8ecd032fe1696054b28f0852b5f03a613681991ace823245ab2f97ed05,
        0x0e67489ecfee0e581dce7db22a383c635886cfb616c3b3bb033bff0c46e9789e,
    );
}
