use std::io::{self, Read};

fn run() -> Result<(), String> {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("embroidery-twin {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let mut text = String::new();
    io::stdin()
        .take(262_145)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() > 262_144 {
        return Err("Design request exceeds 256 KiB.".into());
    }
    let request: embroidery_twin::Request =
        serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let result = embroidery_twin::compile_design(request)?;
    println!(
        "{}",
        serde_json::to_string(&result).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
