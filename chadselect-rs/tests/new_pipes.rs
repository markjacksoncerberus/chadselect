use chadselect::ChadSelect;

fn c() -> ChadSelect {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<html><body>
        <span class="price">$1,299.00</span>
        <span class="mileage">12,345 mi</span>
        <span class="vinline">VIN: 1HGFE2F59PA000001 (verified)</span>
        <a class="dl" href="https://x.com/dealer/42">d</a>
        <span class="file">archive.tar.gz</span>
    </body></html>"#.to_string());
    cs
}

#[test]
fn new_pipe_functions() {
    let cs = c();
    // translate — strip $ and , (comma inside the 'from' arg)
    assert_eq!(cs.select(0, "css:.price >> translate('$,','')"), "1299.00");
    // regex-replace — strip non-digits
    assert_eq!(cs.select(0, "css:.mileage >> regex-replace('[^0-9]','')"), "12345");
    // regex-extract — pull the 17-char VIN
    assert_eq!(cs.select(0, "css:.vinline >> regex-extract('([A-HJ-NPR-Z0-9]{17})')"), "1HGFE2F59PA000001");
    // regex-extract — whole match, no group
    assert_eq!(cs.select(0, "css:.vinline >> regex-extract('[0-9]{4}')"), "0000");
    // substring-after-last — last path segment
    assert_eq!(cs.select(0, "css:.dl >> get-attr('href') >> substring-after-last('/')"), "42");
    // substring-before-last — drop final extension
    assert_eq!(cs.select(0, "css:.file >> substring-before-last('.')"), "archive.tar");
    // replace with a comma arg now works (was buggy)
    assert_eq!(cs.select(0, "css:.price >> replace(',','')"), "$1299.00");
}
