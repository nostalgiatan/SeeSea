//! Judge a saved page: `cargo run -p seesea-vet --example page -- <url> <file.html>`.
//! Without a rank table every site counts as unknown, which is the strictest case.
use seesea_vet::{PageFacts, Vet, VetConfig};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (url, file) = (&a[0], &a[1]);
    let html = std::fs::read_to_string(file).expect("readable file");
    let vet = Vet::new(VetConfig::default());
    let addr = vet.check(url);
    let facts = PageFacts::from_html(&html, url);
    let v = vet.check_page(&addr, &facts);
    println!("{facts:#?}");
    println!(
        "address {} -> page {} ({:?}) {:?}",
        addr.score,
        v.score,
        v.level,
        v.codes()
    );
}
