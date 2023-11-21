use crate::run_command;
use anyhow::{anyhow, Result};
use regex::Regex;
use std::fs::File;
use std::io::{Read, Write};

use std::path::Path;

fn merge_profraw_to_profdata(
    data_path: &Path,
    llvm_major_version: &str,
) -> Result<(), anyhow::Error> {
    let profraw_files: Vec<String> = std::fs::read_dir(&data_path.join("coverage/data"))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_string_lossy() == "profraw" {
                Some(path.to_str()?.to_owned())
            } else {
                None
            }
        })
        .collect();

    if profraw_files.is_empty() {
        return Err(anyhow!(
            "No .profraw files found in the coverage/data directory"
        ));
    }

    let profdata_path = data_path.join("coverage.profdata");
    let output = run_command(
        &format!("llvm-profdata-{}", llvm_major_version),
        &[
            "merge",
            "-sparse",
            &profraw_files.join(" "),
            "-o",
            profdata_path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid path"))?,
        ],
    )?;

    Ok(())
}

fn modify_ll_file(ll_path: &Path) -> Result<()> {
    let mut ll_contents = String::new();

    File::open(&ll_path)
        .expect(&format!("Failed to open LL file {:?}", ll_path))
        .read_to_string(&mut ll_contents)?;

    let modified_ll_contents = Regex::new(r"(?ms)^(define[^\n]*\n).*?^}\s*$")
        .unwrap()
        .replace_all(&ll_contents, "${1}start:\n  unreachable\n}\n")
        .to_string();

    File::create(&ll_path)
        .expect(&format!("Failed to open LL file {:?}", ll_path))
        .write_all(modified_ll_contents.as_bytes())?;

    Ok(())
}

fn generate_ll_object_file(
    wasm_path: &Path,
    data_path: &Path,
    llvm_major_version: &str,
) -> Result<(), anyhow::Error> {
    let ll_path = wasm_path.with_extension("ll");
    let output = run_command(
        &format!("clang-{}", llvm_major_version),
        &[
            ll_path.to_str().unwrap(),
            "-Wno-override-module",
            "-c",
            "-o",
            data_path.join("coverage.ll.o").to_str().unwrap(),
        ],
    )?;

    Ok(())
}

fn generate_coverage_report(
    data_path: &Path,
    coverage_path: &Path,
    llvm_major_version: &str,
) -> Result<(), anyhow::Error> {
    let profdata_path = data_path.join("coverage.profdata");
    let object_file_path = data_path.join("coverage.ll.o");
    let coverage_report_path = coverage_path.join("report");

    let output = run_command(
        &format!("llvm-cov-{}", llvm_major_version),
        &[
            "show",
            "--instr-profile",
            profdata_path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid path"))?,
            object_file_path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid path"))?,
            "--show-instantiations=false",
            "--format=html",
            "--output-dir",
            coverage_report_path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid path"))?,
        ],
    )?;

    println!(
        "Coverage report was successfully generated, it is available in {:?} directory.",
        coverage_report_path
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::dir::get_output_dir;

    use super::*;

    #[test]
    fn test_modify_ll_file() {
        // Copy the tests/fibonacci.ll file to tests/fibonacci-tmp.ll
        let ll_path = Path::new("tests").join("fibonacci.ll");
        let ll_modified_path = Path::new("tests").join("fibonacci-modified.ll");
        let ll_expepcted_path = Path::new("tests").join("fibonacci-modified.ll");
        std::fs::copy(&ll_path, &ll_modified_path).unwrap();

        // Modify the tests/fibonacci-tmp.ll file
        modify_ll_file(&ll_modified_path).unwrap();

        // Compare fibonacci-modified.ll and expected
        let mut ll_modified_contents = String::new();
        let mut ll_expected_contents = String::new();
        File::open(&ll_modified_path)
            .expect(&format!("Failed to open LL file {:?}", ll_modified_path))
            .read_to_string(&mut ll_modified_contents)
            .unwrap();
        File::open(&ll_expepcted_path)
            .expect(&format!("Failed to open LL file {:?}", ll_expepcted_path))
            .read_to_string(&mut ll_expected_contents)
            .unwrap();
        assert_eq!(ll_modified_contents, ll_expected_contents);

        // Clean up
        std::fs::remove_file(&ll_modified_path).unwrap();
    }
}
