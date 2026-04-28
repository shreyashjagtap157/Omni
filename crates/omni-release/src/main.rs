use clap::Parser;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Builder;
use xz2::write::XzEncoder;

#[derive(Parser, Debug)]
#[clap(name = "omni-release")]
struct Cli {
    #[clap(short, long, default_value = ".")]
    path: PathBuf,

    #[clap(short, long)]
    output: Option<PathBuf>,

    #[clap(short, long, default_value = "omni-v0.1.0")]
    version: String,
}

struct Release {
    version: String,
    platform: String,
    arch: String,
}

impl Release {
    fn current(version: String) -> Self {
        let arch = std::env::consts::ARCH;
        let platform = std::env::consts::OS;

        Release {
            version,
            platform: platform.to_string(),
            arch: arch.to_string(),
        }
    }

    fn bundle(&self, source: &Path, output: &Path) -> std::io::Result<()> {
        println!(
            "Bundling {} for {}-{}",
            self.version, self.platform, self.arch
        );

        let output_file = output.join(format!(
            "omni-{}-{}-{}.tar.xz",
            self.version, self.platform, self.arch
        ));

        let file = File::create(&output_file)?;
        let encoder = XzEncoder::new(file, 9);
        let mut tar = Builder::new(encoder);

        let stdlib_dir = source.join("omni");
        if stdlib_dir.exists() {
            tar.append_dir_all("omni", &stdlib_dir)?;
        }

        let examples_dir = source.join("examples");
        if examples_dir.exists() {
            tar.append_dir_all("examples", &examples_dir)?;
        }

        let readme = source.join("README.md");
        if readme.exists() {
            tar.append_path_with_name(&readme, "README.md")?;
        }

        tar.finish()?;
        let encoder = tar.into_inner()?;
        encoder.finish()?;

        println!("Bundle created: {:?}", output_file);
        Ok(())
    }
}

fn main() {
    let cli = Cli::parse();
    let release = Release::current(cli.version);

    let output = cli.output.unwrap_or_else(|| cli.path.join("release"));
    std::fs::create_dir_all(&output).expect("create output dir");

    if let Err(e) = release.bundle(cli.path.as_path(), output.as_path()) {
        eprintln!("Bundle failed: {}", e);
        std::process::exit(1);
    }

    println!("Release bundle created successfully!");
}
