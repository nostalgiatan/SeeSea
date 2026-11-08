//! `cargo run -p seesea-vet --example explain -- <url>...` prints the verdict for each address
//! (without a popularity list, so tiers are unranked).
fn main() {
    let vet = seesea_vet::Vet::new(Default::default());
    for u in std::env::args().skip(1) {
        println!("{}", serde_json::to_string_pretty(&vet.check(&u)).unwrap());
    }
}
