use clap::Parser;
use humansize::{format_size, DECIMAL};
use std::{
    ffi::OsStr,
    fs::File,
    io::{Error, ErrorKind, Read},
    path::Path,
};
use walkdir::WalkDir;
use zip::ZipArchive;

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
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
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
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

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

fn verify_zip(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let directory = path.with_extension("");

    for i in 0..archive.len() {
        let mut expected_file = archive.by_index(i)?;

        if expected_file.is_dir() {
            continue;
        }

        let expected_file_name = expected_file.enclosed_name().ok_or(Error::new(
            ErrorKind::Other,
            "Could not call enclosed_name on file in zip archive",
        ))?;
        let actual_file_path = directory.join(expected_file_name);
        let mut expected_contents = Vec::new();
        let mut actual_contents = Vec::new();

        if !actual_file_path.exists() {
            return Err(Error::new(
                ErrorKind::Other,
                format!("File not found: {}", actual_file_path.display()),
            )
            .into());
        }

        let actual_size = actual_file_path.metadata()?.len();

        if expected_file.size() != actual_size {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "File size mismatch: {} (expected: {}, actual: {})",
                    actual_file_path.display(),
                    expected_file.size(),
                    actual_file_path.metadata()?.len()
                ),
            )
            .into());
        }

        let mut actual_file = File::open(actual_file_path.clone())?;

        expected_file.read_to_end(&mut expected_contents)?;
        actual_file.read_to_end(&mut actual_contents)?;

        if expected_contents != actual_contents {
            return Err(Error::new(
                ErrorKind::Other,
                format!("File contents mismatch: {}", actual_file_path.display()),
            )
            .into());
        }
    }

    Ok(())
}

fn verify(root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ok_count = 0;
    let mut err_count = 0;

    for entry in WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("zip")) {
            continue;
        }

        match verify_zip(path) {
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

fn delete_zip(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    verify_zip(path)?;
    std::fs::remove_file(path)?;
    Ok(())
}

fn delete(root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ok_count = 0;
    let mut err_count = 0;

    for entry in WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("zip")) {
            continue;
        }

        match delete_zip(path) {
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
            verify(root)?;
        }
        Commands::Delete { root } => {
            delete(root)?;
        }
    }

    Ok(())
}
