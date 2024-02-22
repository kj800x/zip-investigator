use humansize::{format_size, DECIMAL};
use std::ffi::OsStr;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let root = &args[1];

    let mut total_extracted: u64 = 0;
    let mut total_compressed: u64 = 0;
    let mut total_savings: u64 = 0;

    for entry in WalkDir::new(root) {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("zip")) {
            continue;
        }

        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut extracted_size: u64 = 0;
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            extracted_size += file.size();
        }
        let compressed_size = std::fs::metadata(path)?.len();

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
