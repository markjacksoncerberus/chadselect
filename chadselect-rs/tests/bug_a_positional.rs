use chadselect::ChadSelect;

fn cs() -> ChadSelect {
    let mut c = ChadSelect::new();
    c.add_html(
        r#"<html><body>
            <table><tr><td>VIN</td><td>VALUE123</td></tr></table>
            <div><span>a</span><span>b</span><span>c</span></div>
        </body></html>"#.to_string(),
    );
    c
}

#[test]
fn positional_predicates_fixed() {
    let c = cs();
    let cases: &[(&str, Vec<&str>)] = &[
        ("xpath://tr/td[1]/text()", vec!["VIN"]),
        ("xpath://tr/td[2]/text()", vec!["VALUE123"]),
        ("xpath://div/span[1]/text()", vec!["a"]),
        ("xpath://div//*[2]/text()", vec!["b"]),
        ("xpath://div/span[position()=2]/text()", vec!["b"]),
        ("xpath://div/span[last()]/text()", vec!["c"]),       // child-axis last()
        ("xpath://div/span[text()='b']/text()", vec!["b"]),   // boolean still works
        ("xpath://div/span/text()", vec!["a","b","c"]),       // no predicate
    ];
    let mut bad = 0;
    for (q, expected) in cases {
        let got = c.query(-1, q);
        let ok = &got == expected;
        if !ok { bad += 1; }
        println!("{:<40} exp {:?} got {:?}{}", q, expected, got, if ok {""} else {"  <-- MISMATCH"});
    }
    assert_eq!(bad, 0, "{bad} cases wrong");
}
