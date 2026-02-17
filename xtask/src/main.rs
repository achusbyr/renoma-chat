use clap::Parser;

mod cli;

fn copy_dir(
    src: impl AsRef<std::path::Path>,
    dst: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Launch { extra_arguments } => {
            let mut cmd = std::process::Command::new("trunk");
            cmd.current_dir("frontend");
            cmd.arg("build");
            cmd.spawn()?.wait()?;

            copy_dir("frontend/styles", "frontend/dist/styles")?;

            let mut cmd = std::process::Command::new("cargo");
            cmd.arg("run")
                .arg("--package")
                .arg("renoma-launcher")
                .arg("--")
                .arg("--dist-dir")
                .arg("frontend/dist");
            for argument in extra_arguments {
                cmd.arg(argument);
            }
            cmd.spawn()?.wait()?;

            Ok(())
        }
        cli::Command::Dist { target_triple } => {
            let mut cmd = std::process::Command::new("cargo");
            cmd.arg("build")
                .arg("--package")
                .arg("renoma-launcher")
                .arg("--release");
            if let Some(target_triple) = target_triple {
                cmd.arg("--target").arg(target_triple);
            }
            cmd.spawn()?.wait()?;

            let mut cmd = std::process::Command::new("trunk");
            cmd.current_dir("frontend");
            cmd.arg("build").arg("--release");
            cmd.spawn()?.wait()?;

            let dist_path = std::path::Path::new("frontend/dist");
            if !dist_path.exists() {
                return Err("frontend/dist directory not found.".into());
            }

            copy_dir("frontend/styles", "frontend/dist/styles")?;

            std::fs::create_dir_all("Renoma/dist")?;
            std::fs::copy("target/release/renoma-launcher", "Renoma/renoma-launcher")?;
            copy_dir(dist_path, "Renoma/dist")?;

            Ok(())
        }
    }
}
