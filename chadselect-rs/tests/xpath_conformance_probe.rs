use chadselect::ChadSelect;

fn c() -> ChadSelect {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<html><body>
        <ul id="list">
          <li class="item" data-n="1">alpha</li>
          <li class="item special" data-n="2">beta</li>
          <li class="item" data-n="3">gamma</li>
        </ul>
        <table><tbody>
          <tr><td>VIN</td><td>ABC</td></tr>
          <tr><td>Mileage</td><td>3500</td></tr>
        </tbody></table>
        <div class="price">  $1,299.00  </div>
    </body></html>"#.to_string());
    cs
}

#[test]
fn probe() {
    let cs = c();
    let cases: &[(&str, Vec<&str>)] = &[
        ("xpath://li/text()", vec!["alpha","beta","gamma"]),
        ("xpath://li[@class='item']/text()", vec!["alpha","gamma"]),
        ("xpath://li[contains(@class,'special')]/text()", vec!["beta"]),
        ("xpath://li[@data-n='2']/text()", vec!["beta"]),
        ("xpath://li[1]/text()", vec!["alpha"]),
        ("xpath://li[3]/text()", vec!["gamma"]),
        ("xpath://ul/li[last()]/text()", vec!["gamma"]),
        ("xpath://li[position()=2]/text()", vec!["beta"]),
        ("xpath://*[@id='list']/li[2]/text()", vec!["beta"]),
        ("xpath://li/@data-n", vec!["1","2","3"]),
        ("xpath://tr[2]/td[2]/text()", vec!["3500"]),
        ("xpath://tr/td[1]/text()", vec!["VIN","Mileage"]),
        ("xpath:normalize-space(//div[@class='price'])", vec!["$1,299.00"]),
        ("xpath:count(//li)", vec!["3"]),
        ("xpath://li[starts-with(text(),'be')]/text()", vec!["beta"]),
        ("xpath://li[@data-n='2']/following-sibling::li/text()", vec!["gamma"]),
        ("xpath://li[@data-n='2']/preceding-sibling::li/text()", vec!["alpha"]),
        ("xpath://td[.='VIN']/following-sibling::td/text()", vec!["ABC"]),
        ("xpath://li[@class='item'][2]/text()", vec!["gamma"]),
        ("xpath://ul/li[last()-1]/text()", vec!["beta"]),
        ("xpath:string(//li[1])", vec!["alpha"]),
        ("xpath://li[2]/@class", vec!["item special"]),
    ];
    let mut bad = vec![];
    for (q, exp) in cases {
        let got = cs.query(-1, q);
        let ok = &got == exp;
        println!("{:<55} {} exp {:?} got {:?}", q, if ok {"OK "} else {"!! "}, exp, got);
        if !ok { bad.push(*q); }
    }
    println!("\n{} mismatches", bad.len());
    for q in &bad { println!("  MISMATCH: {q}"); }
}
