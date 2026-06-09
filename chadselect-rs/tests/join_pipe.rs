use chadselect::ChadSelect;

fn cs() -> ChadSelect {
    let mut c = ChadSelect::new();
    c.add_html(r#"<html><body>
        <nav class="crumbs"><a>Home</a><a>Cars</a><a>Civic</a></nav>
        <ul class="specs"><li>2.0L</li><li>Turbo</li><li>AWD</li></ul>
        <p class="opt _optInteriorColor">Interior Color: Black</p>
        <p class="opt _optInterior">Interior: Leather</p>
    </body></html>"#.to_string());
    c
}

#[test]
fn join_pipe_basics() {
    let c = cs();
    assert_eq!(c.select(-1, "css:.crumbs a >> join(' / ')"), "Home / Cars / Civic");
    assert_eq!(c.select(-1, "css:.specs li >> concat('-')"), "2.0L-Turbo-AWD"); // alias
    assert_eq!(c.select(-1, "css:.specs li >> join()"), "2.0LTurboAWD");        // empty sep
    assert_eq!(c.select(-1, "css:.crumbs a >> uppercase() >> join('>')"), "HOME>CARS>CIVIC");
    assert_eq!(c.select(-1, "xpath://ul[@class='specs']/li/text() >> join(' ')"), "2.0L Turbo AWD");
}

#[test]
fn join_replaces_deep_xpath_concat() {
    // The old deeply-nested xpath concat(...) for interior, as flat CSS + pipes.
    let c = cs();
    let q = "css:p[class*='_optInterior'] \
             >> replace('Interior Color:','') >> replace('Interior:','') \
             >> normalize-space() >> join(' ')";
    assert_eq!(c.select(-1, q), "Black Leather");
}
