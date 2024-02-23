use clap::Parser;
use humansize::{format_size, DECIMAL};
use std::{ffi::OsStr, path::Path};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Investigate { root: String },
    Extract { root: String },
    Verify { root: String },
    Delete { root: String },
}

struct InvestigateOk {
    extracted_size: u64,
    compressed_size: u64,
}

fn investigate_zip(path: &Path) -> Result<InvestigateOk, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut extracted_size: u64 = 0;
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        extracted_size += file.size();
    }
    let compressed_size = std::fs::metadata(path)?.len();

    Ok(InvestigateOk {
        extracted_size,
        compressed_size,
    })
}

fn investigate(root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut total_extracted: u64 = 0;
    let mut total_compressed: u64 = 0;
    let mut total_savings: u64 = 0;

    for entry in WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("zip")) {
            continue;
        }

        match investigate_zip(path) {
            Ok(InvestigateOk {
                extracted_size,
                compressed_size,
            }) => {
                total_extracted += extracted_size;
                total_compressed += compressed_size;
                total_savings += extracted_size - compressed_size;

                println!();
                println!("File: {}", path.as_os_str().to_str().unwrap());
                println!("Extracted size : {}", format_size(extracted_size, DECIMAL),);
                println!(
                    "Compressed size: {} ({:.2}%)",
                    format_size(compressed_size, DECIMAL),
                    (compressed_size as f64 / extracted_size as f64) * 100.0
                );
                println!(
                    "Savings        : {}",
                    format_size(extracted_size - compressed_size, DECIMAL)
                );
            }
            Err(e) => {
                println!();
                println!("File: {}", path.as_os_str().to_str().unwrap());
                println!("Error: {}", e);
            }
        }
    }

    println!();
    println!(
        "Total extracted size : {} ({})",
        total_extracted,
        format_size(total_extracted, DECIMAL)
    );
    println!(
        "Total compressed size: {} ({}) - ({:.2}%)",
        total_compressed,
        format_size(total_compressed, DECIMAL),
        (total_compressed as f64 / total_extracted as f64) * 100.0
    );
    println!(
        "Total savings        : {} ({})",
        total_savings,
        format_size(total_savings, DECIMAL)
    );

    Ok(())
}

fn extract_zip(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // extract the zip into a directory with the same name (without the .zip extension)
    let directory = path.with_extension("");
    if directory.exists() {
        println!(
            "WARN : Directory already exists, overwriting: {}",
            directory.display()
        );
    }
    std::fs::create_dir_all(&directory)?;

    archive.extract(directory)?;

    Ok(())
}

fn extract(root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ok_count = 0;
    let mut err_count = 0;
    for entry in WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("zip")) {
            continue;
        }

        match extract_zip(path) {
            Ok(()) => {
                println!("OK   : {}", path.as_os_str().to_str().unwrap());
                ok_count += 1;
            }
            Err(e) => {
                println!("ERR  : {}", path.as_os_str().to_str().unwrap());
                println!("Error: {}", e);
                err_count += 1;
            }
        }
    }

    println!();
    println!("OK  count: {}", ok_count);
    println!("ERR count: {}", err_count);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Investigate { root } => {
            investigate(root)?;
        }
        Commands::Extract { root } => {
            extract(root)?;
        }
        Commands::Verify { root } => {
            println!("Verify todo, root: {}", root);
        }
        Commands::Delete { root } => {
            println!("Delete todo, root: {}", root);
        }
    }

    Ok(())
}
