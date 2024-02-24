use clap::Parser;
use indicatif::{HumanBytes, MultiProgress, ProgressBar};
use itertools::Itertools;
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
    let zips = discover_zips(root);
    let bar = ProgressBar::new(zips.len() as u64);

    for entry in zips {
        let path = entry.path();
        match investigate_zip(path) {
            Ok(InvestigateOk {
                extracted_size,
                compressed_size,
            }) => {
                total_extracted += extracted_size;
                total_compressed += compressed_size;
                total_savings += extracted_size - compressed_size;

                bar.println("");
                bar.println(format!("File: {}", path.as_os_str().to_str().unwrap()));
                bar.println(format!("Extracted size : {}", HumanBytes(extracted_size)));
                bar.println(format!(
                    "Compressed size: {} ({:.2}%)",
                    HumanBytes(compressed_size),
                    (compressed_size as f64 / extracted_size as f64) * 100.0
                ));
                bar.println(format!(
                    "Savings        : {}",
                    HumanBytes(extracted_size - compressed_size)
                ));
            }
            Err(e) => {
                bar.println(format!(""));
                bar.println(format!("File: {}", path.as_os_str().to_str().unwrap()));
                bar.println(format!("Error: {}", e));
            }
        }
        bar.inc(1);
    }

    bar.finish_and_clear();

    println!();
    println!(
        "Total extracted size : {} ({})",
        total_extracted,
        HumanBytes(total_extracted)
    );
    println!(
        "Total compressed size: {} ({}) - ({:.2}%)",
        total_compressed,
        HumanBytes(total_compressed),
        (total_compressed as f64 / total_extracted as f64) * 100.0
    );
    println!(
        "Total savings        : {} ({})",
        total_savings,
        HumanBytes(total_savings)
    );

    Ok(())
}

fn discover_zips(root: &str) -> Vec<walkdir::DirEntry> {
    println!("Discovering zip files...");
    let spinner = ProgressBar::new_spinner();
    let zips = WalkDir::new(root)
        .into_iter()
        .filter_map(|x| -> Option<walkdir::DirEntry> {
            spinner.tick();
            let path = x.as_ref().ok()?.path();
            if path.extension() == Some(OsStr::new("zip")) {
                Some(x.unwrap())
            } else {
                None
            }
        })
        .collect_vec();
    spinner.finish_and_clear();

    zips
}

fn extract_zip(path: &Path, bar: &ProgressBar) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    // extract the zip into a directory with the same name (without the .zip extension)
    let directory = path.with_extension("");
    if directory.exists() {
        bar.println(format!(
            "WARN : Directory already exists, overwriting: {}",
            directory.display()
        ));
    }
    std::fs::create_dir_all(&directory)?;

    archive.extract(directory)?;

    Ok(())
}

fn extract(root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ok_count = 0;
    let mut err_count = 0;
    let zips = discover_zips(root);
    println!("Extracting discovered zip files...");
    let bar = ProgressBar::new(zips.len() as u64);
    for entry in zips {
        let path = entry.path();
        match extract_zip(path, &bar) {
            Ok(()) => {
                bar.println(format!("OK   : {}", path.as_os_str().to_str().unwrap()));
                ok_count += 1;
            }
            Err(e) => {
                bar.println(format!("ERR  : {}", path.as_os_str().to_str().unwrap()));
                bar.println(format!("Error: {}", e));
                err_count += 1;
            }
        }
        bar.inc(1);
    }
    bar.finish_and_clear();

    println!();
    println!("OK  count: {}", ok_count);
    println!("ERR count: {}", err_count);

    Ok(())
}

fn verify_zip(
    path: &Path,
    bar_container: &MultiProgress,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let directory = path.with_extension("");

    let inner_bar = bar_container.add(ProgressBar::new(archive.len() as u64));
    for i in 0..archive.len() {
        inner_bar.inc(1);
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

    inner_bar.finish_and_clear();
    bar_container.remove(&inner_bar);

    Ok(())
}

fn verify(root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ok_count = 0;
    let mut err_count = 0;

    let zips = discover_zips(root);
    println!("Verifying zip files were extracted correctly...");
    let bar_container = MultiProgress::new();
    let bar = bar_container.add(ProgressBar::new(zips.len() as u64));
    bar.tick();
    for entry in zips {
        let path = entry.path();
        match verify_zip(path, &bar_container) {
            Ok(()) => {
                bar_container.println(format!("OK   : {}", path.as_os_str().to_str().unwrap()))?;
                ok_count += 1;
            }
            Err(e) => {
                bar_container.println(format!("ERR  : {}", path.as_os_str().to_str().unwrap()))?;
                bar_container.println(format!("Error: {}", e))?;
                err_count += 1;
            }
        }
        bar.inc(1);
    }
    bar.finish_and_clear();
    bar_container.clear()?;

    println!();
    println!("OK  count: {}", ok_count);
    println!("ERR count: {}", err_count);

    Ok(())
}

fn delete_zip(
    path: &Path,
    bar_container: &MultiProgress,
) -> Result<(), Box<dyn std::error::Error>> {
    verify_zip(path, bar_container)?;
    std::fs::remove_file(path)?;
    Ok(())
}

fn delete(root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ok_count = 0;
    let mut err_count = 0;

    let zips = discover_zips(root);
    println!("Deleting zip files that are already extracted...");
    let bar_container = MultiProgress::new();
    let bar = bar_container.add(ProgressBar::new(zips.len() as u64));
    bar.tick();
    for entry in zips {
        let path = entry.path();
        match delete_zip(path, &bar_container) {
            Ok(()) => {
                bar.println(format!("OK   : {}", path.as_os_str().to_str().unwrap()));
                ok_count += 1;
            }
            Err(e) => {
                bar.println(format!("ERR  : {}", path.as_os_str().to_str().unwrap()));
                bar.println(format!("Error: {}", e));
                err_count += 1;
            }
        }
        bar.inc(1);
    }
    bar.finish_and_clear();
    bar_container.remove(&bar);
    bar_container.clear()?;

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
