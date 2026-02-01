use clap::Parser;
use std::path::{Path, PathBuf};

/// Files that should never be updated from the instance
const IGNORE_LIST: &[&str] = &[".release-please-manifest.json"];

/// Given a template type and repository instance,
/// update all existing template files with the
/// instance content.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Template to update.
    #[arg(short, long)]
    template: String,

    /// Instance to reference.
    #[arg(short, long)]
    instance: String,
}

fn collect_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                files.extend(collect_files(&path)?);
            } else {
                files.push(path);
            }
        }
    }

    Ok(files)
}

fn main() {
    let args = Args::parse();

    // Get the workspace root (where the main Cargo.toml is)
    // Assume we're run from the workspace root via `cargo xtask`
    let workspace_root = std::env::current_dir().expect("failed to get current directory");

    // Resolve template path relative to workspace root
    let template_path = workspace_root.join(&args.template);
    if !template_path.is_dir() {
        panic!(
            "template '{}' does not exist at path '{}'",
            args.template,
            template_path.display()
        );
    }

    // Ensure instance is a directory
    let instance_path = Path::new(&args.instance);
    if !instance_path.is_dir() {
        panic!("instance repository '{}' does not exist", args.instance);
    }

    // Collect all files in the template directory recursively
    let template_files =
        collect_files(&template_path).expect("failed to read template directory contents");

    println!(
        "Found {} files in template '{}'",
        template_files.len(),
        args.template
    );

    // For each file in the template, find the corresponding file in the instance
    for template_file in template_files {
        // Get the relative path from the template root
        let relative_path = template_file
            .strip_prefix(&template_path)
            .expect("failed to get relative path");

        // Find the corresponding file in the instance
        let instance_file = instance_path.join(relative_path);

        // Check if this file should be ignored
        let file_name = relative_path.to_str().unwrap_or("");
        if IGNORE_LIST
            .iter()
            .any(|&ignore| file_name.ends_with(ignore))
        {
            println!("Skipping ignored file '{}'", relative_path.display());
            continue;
        }

        if !instance_file.exists() {
            println!(
                "Warning: instance file '{}' does not exist",
                instance_file.display()
            );
        } else {
            println!(
                "Copying '{}' -> '{}'",
                instance_file.display(),
                template_file.display()
            );
            std::fs::copy(&instance_file, &template_file).expect(&format!(
                "failed to copy file to '{}'",
                template_file.display()
            ));
        }
    }

    println!("Template update complete!");
}
