//! Print the metadata and tensor table of a GGUF file: `cargo run -p seesea-embed --example gguf_info -- FILE`
use candle_core::quantized::gguf_file::{Content, Value};

fn main() {
    let path = std::env::args().nth(1).expect("path to a .gguf file");
    let mut f = std::fs::File::open(&path).unwrap();
    let c = Content::read(&mut f).unwrap();
    let mut keys: Vec<_> = c.metadata.iter().collect();
    keys.sort_by_key(|(k, _)| k.to_string());
    for (k, v) in keys {
        let shown = match v {
            Value::Array(a) => format!("array[{}]", a.len()),
            Value::String(s) => format!("{:?}", s.chars().take(80).collect::<String>()),
            other => format!("{other:?}"),
        };
        println!("{k} = {shown}");
    }
    let mut names: Vec<_> = c.tensor_infos.iter().collect();
    names.sort_by_key(|(k, _)| k.to_string());
    for (k, t) in names.iter().take(400) {
        println!("{k} {:?} {:?}", t.shape, t.ggml_dtype);
    }
    println!("{} tensors", c.tensor_infos.len());
}
