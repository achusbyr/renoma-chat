use clap::Parser;

mod cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Launch { extra_arguments } => {
            let mut cmd = std::process::Command::new("trunk");
            cmd.current_dir("frontend");
            cmd.arg("build");
            cmd.spawn()?.wait()?;

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

            std::fs::create_dir_all("Renoma/dist")?;
            std::fs::copy("target/release/renoma-launcher", "Renoma/renoma-launcher")?;

            let dist_path = std::path::Path::new("frontend/dist");
            if !dist_path.exists() {
                return Err("frontend/dist directory not found. Did trunk build fail?".into());
            }

            for entry in std::fs::read_dir(dist_path)? {
                let entry = entry?;
                let file_name = entry.file_name();
                std::fs::copy(
                    entry.path(),
                    format!(
                        "Renoma/dist/{}",
                        file_name.to_str().ok_or("Invalid file name")?
                    ),
                )?;
            }

            Ok(())
        }
    }
}
