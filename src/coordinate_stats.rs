use anyhow::Result;
// Add encoding_rs imports
use encoding_rs::SHIFT_JIS;
use encoding_rs_io::DecodeReaderBytesBuilder;
// Add indicatif imports
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use zip::ZipArchive;

/// Extracts the CSV search list file from the zip archive and calculates
/// the number of parcels that exist in each nested zip file.
/// Returns a HashMap with the file name as the key and the number of parcels as the value.
fn process_zip_csv_search_list<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<HashMap<String, usize>> {
    // file is in the format `...-search-list.csv`, let's find it first
    let file_idx = (0..archive.len())
        .find(|&i| {
            let file = archive.by_index(i).unwrap();
            file.name().ends_with("-search-list.csv")
        })
        .ok_or_else(|| anyhow::anyhow!("No search list CSV found in the zip"))?;
    let mut file = archive.by_index(file_idx)?;

    let mut decoder = DecodeReaderBytesBuilder::new()
        .encoding(Some(SHIFT_JIS))
        .build(&mut file);
    let mut content = String::new();
    decoder.read_to_string(&mut content)?;

    let mut search_list = HashMap::new();

    let re = Regex::new(r"^([\d-]+\.zip),")?;
    for line in content.lines() {
        if let Some(captures) = re.captures(line) {
            if let Some(file_name) = captures.get(1) {
                let file_name = file_name.as_str().to_string();
                let count = search_list.entry(file_name).or_insert(0);
                *count += 1;
            }
        }
    }

    Ok(search_list)
}

fn process_zip_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
    regex: &Regex,
) -> Result<(usize, usize)> {
    let mut file = archive.by_index(index)?;
    let mut local_arbitrary_count = 0;
    let mut local_public_count = 0;

    if file.is_dir() {
        return Ok((0, 0));
    }

    let file_name = file.name().to_string();

    if file_name.ends_with(".zip") {
        // println!("Processing nested zip file: {}", file_name);
        let mut inner_zip_data = Vec::new();
        file.read_to_end(&mut inner_zip_data)?;

        let cursor = Cursor::new(inner_zip_data);
        let mut inner_archive = match ZipArchive::new(cursor) {
            Ok(archive) => archive,
            Err(e) => {
                eprintln!("Failed to open nested zip {}: {}", file_name, e);
                return Ok((0, 0));
            }
        };

        // Get the length before the loop
        let inner_len = inner_archive.len();
        for i in 0..inner_len {
            match process_zip_entry(&mut inner_archive, i, regex) {
                Ok((nested_arbitrary, nested_public)) => {
                    local_arbitrary_count += nested_arbitrary;
                    local_public_count += nested_public;
                }
                Err(e) => {
                    eprintln!(
                        "Error processing entry {} in nested zip {}: {}",
                        i, file_name, e
                    );
                }
            }
        }
    } else if file_name.ends_with(".xml") {
        const MAX_READ_SIZE: usize = 4096;
        let mut buffer = vec![0; MAX_READ_SIZE];
        let bytes_read = file.take(MAX_READ_SIZE as u64).read(&mut buffer)?;
        buffer.truncate(bytes_read);

        let content = String::from_utf8_lossy(&buffer);

        if let Some(captures) = regex.captures(&content) {
            if let Some(value) = captures.get(1) {
                let coord_system = value.as_str();
                if coord_system == "任意座標系" {
                    local_arbitrary_count += 1;
                } else if coord_system.contains("公共座標") {
                    local_public_count += 1;
                } else {
                    eprintln!(
                        "Unknown coordinate system in file {}: {}",
                        file_name, coord_system
                    );
                }
            }
        }
    }

    Ok((local_arbitrary_count, local_public_count))
}

fn process_zip_file(zip_path: &Path, regex: &Regex) -> Result<(usize, usize)> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut total_arbitrary = 0;
    let mut total_public = 0;

    // Process the CSV search list file
    let search_list = process_zip_csv_search_list(&mut archive)?;
    if search_list.is_empty() {
        eprintln!("No search list found in the zip file.");
        anyhow::bail!("No search list found in the zip file.");
    }

    // Get the length before the loop
    let archive_len = archive.len();
    for i in 0..archive_len {
        let file_name = archive.by_index(i)?.name().to_string();
        let count_in_search_list = search_list.get(&file_name).unwrap_or(&0);
        if *count_in_search_list == 0 {
            // eprintln!("Skipping file {}: not in search list", file_name);
            continue;
        }
        match process_zip_entry(&mut archive, i, regex) {
            Ok((entry_arbitrary, entry_public)) => {
                total_arbitrary += entry_arbitrary * count_in_search_list;
                total_public += entry_public * count_in_search_list;
            }
            Err(e) => {
                eprintln!("Error processing entry {} in {:?}: {}", i, zip_path, e);
            }
        }
    }
    Ok((total_arbitrary, total_public))
}

pub fn run_coordinate_stats(dir_path: &Path) -> Result<()> {
    println!("Scanning directory for zip files: {:?}", dir_path);

    let re = Regex::new(r"<座標系>(.*?)</座標系>").expect("Failed to compile regex");

    let arbitrary_count = AtomicUsize::new(0);
    let public_count = AtomicUsize::new(0);

    // Collect zip file paths first to know the total count for the progress bar
    let zip_files: Vec<_> = fs::read_dir(dir_path)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().map_or(false, |ext| ext == "zip"))
        .collect();

    if zip_files.is_empty() {
        println!("No zip files found in the directory.");
        return Ok(());
    }

    println!(
        "Found {} zip files. Starting parallel processing...",
        zip_files.len()
    );

    // Create and configure the progress bar
    let pb = ProgressBar::new(zip_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .expect("Failed to set progress bar template")
            .progress_chars("#>-"),
    );

    // Process zip files in parallel with progress bar integration
    zip_files
        .into_par_iter()
        .progress_with(pb.clone()) // Wrap the iterator for progress updates
        .for_each(|path| match process_zip_file(&path, &re) {
            Ok((local_arbitrary, local_public)) => {
                arbitrary_count.fetch_add(local_arbitrary, Ordering::Relaxed);
                public_count.fetch_add(local_public, Ordering::Relaxed);
            }
            Err(e) => {
                pb.println(format!(
                    "Error processing zip file {:?}: {}",
                    path.file_name().unwrap_or_else(|| path.as_os_str()),
                    e
                ));
            }
        });

    pb.finish_with_message("Finished processing all zip files."); // Finalize the progress bar

    let final_arbitrary = arbitrary_count.load(Ordering::SeqCst);
    let final_public = public_count.load(Ordering::SeqCst);

    println!("\n--- Coordinate System Stats ---");
    println!("任意座標系 count: {}", final_arbitrary);
    println!("公共座標系 count: {}", final_public);
    println!("------------------------------");

    Ok(())
}
