use clap::Parser;

mod cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Launch => {
            let mut cmd = std::process::Command::new("trunk");
            cmd.current_dir(std::fs::canonicalize("frontend")?);
            cmd.arg("build");
            cmd.spawn()?.wait()?;

            let mut cmd = std::process::Command::new("cargo");
            cmd.arg("run")
                .arg("--package")
                .arg("renoma-launcher")
                .arg("--")
                .arg("--dist-dir")
                .arg("frontend/dist");
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
            cmd.current_dir(std::fs::canonicalize("frontend")?);
            cmd.arg("build").arg("--release");
            cmd.spawn()?.wait()?;

            std::fs::create_dir_all("Renoma/dist")?;
            std::fs::copy("target/release/renoma-launcher", "Renoma/renoma-launcher")?;
            for file in std::fs::read_dir("frontend/dist")? {
                let file = file?;
                std::fs::copy(
                    file.path(),
                    format!("Renoma/dist/{}", file.file_name().to_str().unwrap()),
                )?;
            }

            Ok(())
        }
    }
}
