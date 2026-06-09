use chadselect::ChadSelect;

fn nested(depth: usize) -> String {
    // //a[a[a[ ... ]]] — `depth` nested predicates
    let mut q = String::from("xpath://a");
    for _ in 0..depth { q.push_str("[a"); }
    for _ in 0..depth { q.push(']'); }
    q
}

#[test]
fn deep_query_does_not_crash() {
    let mut c = ChadSelect::new();
    c.add_html("<html><body><div><span>x</span></div></body></html>".to_string());

    // ~15 levels overflowed a 2 MiB stack on 0.3.0. These run on the test
    // harness's default-stack thread; if the fix works, none crash the process.
    for depth in [10usize, 30, 60, 120] {
        let got = c.query(-1, &nested(depth));
        println!("depth {:>3} -> {} results (no crash)", depth, got.len());
        assert!(got.is_empty()); // no <a> in the doc
    }

    // Absurd depth is refused gracefully (empty), not crashed.
    let got = c.query(-1, &nested(5000));
    println!("depth 5000 -> {} results (refused gracefully)", got.len());
    assert!(got.is_empty());
}
