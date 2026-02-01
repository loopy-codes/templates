///! Development tasks for this Cargo workspace.
use cargo_metadata::MetadataCommand;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use toml_edit::DocumentMut;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Sync development dependencies
    Sync,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.command {
        Command::Sync => sync()?,
    }
    Ok(())
}

fn sync() -> Result<(), Box<dyn std::error::Error>> {
    let metadata = MetadataCommand::new()
        .exec()
        .expect("failed to generate workspace metadata");

    let member_versions: HashMap<_, _> = metadata
        .workspace_packages()
        .iter()
        .map(|p| (p.name.clone(), p.version.clone()))
        .collect();

    println!("Found {} workspace members", member_versions.len());
    for (name, version) in &member_versions {
        println!("  {} = {}", name, version);
    }

    for package in &metadata.workspace_packages() {
        let manifest_path = &package.manifest_path;
        let cargo_toml_content = fs::read_to_string(manifest_path)?;
        let mut doc = cargo_toml_content.parse::<DocumentMut>()?;
        let mut modified = false;

        for dep_section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(deps) = doc.get_mut(dep_section).and_then(|d| d.as_table_mut()) else {
                continue;
            };

            for (dep_name, dep_value) in deps.iter_mut() {
                let Some(member_version) = member_versions.get(dep_name.get()) else {
                    continue;
                };

                let updated = if let Some(table) = dep_value.as_inline_table_mut() {
                    update_version(
                        table,
                        member_version,
                        dep_name.get(),
                        &package.name,
                        dep_section,
                    )
                } else if let Some(table) = dep_value.as_table_mut() {
                    update_version(
                        table,
                        member_version,
                        dep_name.get(),
                        &package.name,
                        dep_section,
                    )
                } else {
                    false
                };

                if updated {
                    modified = true;
                }
            }
        }

        if modified {
            fs::write(manifest_path, doc.to_string())?;
            println!("Wrote changes to {}", manifest_path);
        }
    }

    println!("Sync complete!");
    Ok(())
}

fn update_version<T: toml_edit::TableLike>(
    table: &mut T,
    member_version: &cargo_metadata::semver::Version,
    dep_name: &str,
    package_name: &str,
    dep_section: &str,
) -> bool {
    if !table.contains_key("version") || table.contains_key("workspace") {
        return false;
    }

    let Some(old_version_str) = table.get("version").and_then(|v| v.as_str()) else {
        return false;
    };

    let Ok(old_version) = old_version_str.parse::<cargo_metadata::semver::Version>() else {
        return false;
    };

    if &old_version == member_version {
        return false;
    }

    table.insert("version", toml_edit::value(member_version.to_string()));
    println!(
        "Updated {} in {}: {} -> {} in [{}]",
        dep_name, package_name, old_version, member_version, dep_section
    );
    true
}
