use std::error::Error;
fn main() -> Result<(), Box<dyn Error>> {
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "--tuning", "EADG", "sparse_bass.json"])
        .env("RUST_BACKTRACE", "1")
        .output()?;
    println!("status: {}", output.status);
    Ok(())
}
