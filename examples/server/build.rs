use std::path::Path;

fn main() -> anyhow::Result<()> {
    qm_role_build::generate(&Path::new("./templates/roles.md"))?;
    Ok(())
}
