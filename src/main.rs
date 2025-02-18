use clap::Parser;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about, long_about)]
struct Args {
    // path of the folder containing the avif files
    #[arg(short, long)]
    avif_destination: String,

    // path of the folder containing the png files
    #[arg(short, long)]
    png_source: String,

    // Simulate deletion without modifying files
    #[arg(long)]
    dry_run: bool,
}

fn collect_files(dir: &Path) -> std::io::Result<HashMap<String, Vec<PathBuf>>> {
    let entries = fs::read_dir(dir)?;

    let pairs: Vec<(String, PathBuf)> = entries
        .par_bridge()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }

            entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| (s.to_lowercase(), path))
        })
        .collect();

    let mut map = HashMap::new();
    for (stem, path) in pairs {
        map.entry(stem).or_insert_with(Vec::new).push(path);
    }

    Ok(map)
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let (avif_dir, png_dir) = (
        Path::new(&args.avif_destination),
        Path::new(&args.png_source),
    );

    // Parallel directory scanning
    let (avif_files, png_files) =
        rayon::join(|| collect_files(avif_dir), || collect_files(png_dir));

    let (avif_files, png_files) = (avif_files?, png_files?);

    // Identify files to remove
    let to_delete: Vec<PathBuf> = png_files
        .into_iter()
        .filter(|(stem, _)| avif_files.contains_key(stem))
        .flat_map(|(_, paths)| paths)
        .collect();

    if args.dry_run {
        for (i, path) in to_delete.iter().enumerate() {
            println!("{}- `{}`", i + 1, path.display());
        }
        println!(
            "\nDry run: Would delete {} already processed png files",
            to_delete.len()
        );
    } else {
        // Parallel deletion with error handling
        let delete_count = to_delete.len();
        let errors: Vec<_> = to_delete
            .par_iter()
            .filter_map(|path| fs::remove_file(path).err())
            .collect();

        println!("Successfully deleted {} files", delete_count - errors.len());
        if !errors.is_empty() {
            eprintln!("Failed to delete {} files:", errors.len());
            for err in errors {
                eprintln!("- {}", err);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    // TODO: Add tests
}
