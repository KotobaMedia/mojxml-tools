# MojXML Tools

A collection of tools for processing and analyzing MOJ XML data archives, commonly used for Japanese cadastral information.

## Features

*   **Coordinate System Statistics:** Scans directories containing MOJ ZIP archives (including nested ZIPs) and counts the occurrences of "任意座標系" and "公共座標系" specified within the XML files. It utilizes the `-search-list.csv` file within the main ZIP archive to determine which nested ZIPs contain relevant parcel data.

## Prerequisites

*   Rust programming language and Cargo package manager. ([Install Rust](https://www.rust-lang.org/tools/install))

## Input Data Format

The tool expects a directory containing one or more ZIP archives. Each main ZIP archive should contain:
1.  A CSV file named `...-search-list.csv` (encoded in Shift-JIS). This file lists the nested ZIP files and is used to count parcels associated with each nested archive.
2.  Nested ZIP archives (e.g., `13101-0000-0000.zip`) referenced in the search list CSV.
3.  These nested ZIP archives contain the actual MOJ XML files (`.xml`). The tool reads the `<座標系>` tag within these XML files to determine the coordinate system type.

**Note:** You can download the required MOJ ZIP archives using tools like [amx-project/dl-tool](https://github.com/amx-project/dl-tool).

## Building

Navigate to the project directory in your terminal and run:

```bash
cargo build --release
```

This will create an executable file in the `target/release/` directory.

## Usage

To run the coordinate statistics tool, execute the compiled binary, providing the path to the directory containing your MOJ ZIP archives:

```bash
./target/release/mojxml-tools coordinate-stats --path <path_to_your_zip_directory>
```

Replace `<path_to_your_zip_directory>` with the actual path to your data.

The tool will scan the directory, process each ZIP file in parallel (showing a progress bar), and output the total counts for each coordinate system type found.

Example Output:

```
Scanning directory for zip files: "path/to/your/zip/directory"
Found 15 zip files. Starting parallel processing...
[00:00:05] [########################################] 15/15 (00:00:00)
Finished processing all zip files.

--- Coordinate System Stats ---
任意座標系 count: 12345
公共座標系 count: 67890
------------------------------
```

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues.

## License

This project is licensed under the [MIT License](LICENSE) (assuming MIT, add a LICENSE file if needed).
