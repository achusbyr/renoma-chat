use clap::Parser;

mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Launch => {
            let mut cmd = tokio::process::Command::new("trunk");
            cmd.current_dir(std::fs::canonicalize("frontend")?);
            cmd.arg("build");
            cmd.spawn()?.wait().await?;

            let mut cmd = tokio::process::Command::new("cargo");
            cmd.arg("run")
                .arg("--package")
                .arg("renoma-launcher")
                .arg("--")
                .arg("--dist-dir")
                .arg("frontend/dist");
            cmd.spawn()?.wait().await?;

            Ok(())
        }
        cli::Command::Dist { target_triple } => {
            let mut cmd = tokio::process::Command::new("cargo");
            cmd.arg("build")
                .arg("--package")
                .arg("renoma-launcher")
                .arg("--release");
            if let Some(target_triple) = target_triple {
                cmd.arg("--target").arg(target_triple);
            }
            cmd.spawn()?.wait().await?;

            let mut cmd = tokio::process::Command::new("trunk");
            cmd.current_dir(std::fs::canonicalize("frontend")?);
            cmd.arg("build").arg("--release");
            cmd.spawn()?.wait().await?;

            tokio::fs::create_dir("Renoma").await?;
            tokio::fs::create_dir("Renoma/dist").await?;
            tokio::fs::copy(
                "renoma-launcher/target/release/renoma-launcher",
                "Renoma/renoma-launcher",
            )
            .await?;
            while let Some(file) = tokio::fs::read_dir("frontend/dist")
                .await?
                .next_entry()
                .await?
            {
                tokio::fs::copy(
                    file.path(),
                    format!("Renoma/dist/{}", file.file_name().to_str().unwrap()),
                )
                .await?;
            }

            Ok(())
        }
    }
}
