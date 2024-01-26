use std::path::{Path, PathBuf};

pub fn generate(input_file_path: &Path) -> anyhow::Result<()> {
    let out = input_file_path.with_extension("rs");
    let file_name = out.file_name().ok_or(anyhow::anyhow!("invalid input filename"))?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let out_file_path = out_dir.join(file_name);
    std::fs::write(out_file_path, r#"
#[allow(dead_code)]
fn roles() {}"#)?;
    Ok(())
}
