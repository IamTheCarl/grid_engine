// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use argh::FromArgs;
use colored::*;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// Yes I  know it seems a little silly to be using a library for writing embedded probe software, but where else can I find
// a maintained function that does exactly this?
use probe_rs_cli_util::build_artifact;

const META_HEADER_VERSION: u16 = 0;

#[derive(FromArgs, PartialEq, Debug)]
/// This is a command line tool meant to assist in the creation of content for the Grid Engine.
struct Arguments {
    #[argh(subcommand)]
    command: SubCommands,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubCommands {
    Pack(Pack),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "pack")]
/// Package your code and assets into a package that can be loaded by the grid engine.
struct Pack {
    #[argh(option)]
    /// optionally specify the path of the cargo project you wish to pack. If not specified, assumes the current directory.
    path: Option<PathBuf>,

    #[argh(option)]
    /// optionally specify the path to drop the mod file into. If not specified, will default to the workspace's target directory.
    target_dir: Option<PathBuf>,
}

fn main() {
    let arguments: Arguments = argh::from_env();

    match arguments.command {
        SubCommands::Pack(arguments) => pack_project(&arguments),
    }
}

/// Produce a package for the project.
fn pack_project(arguments: &Pack) {
    match get_project_dir(arguments.path.clone()) {
        Ok(project_dir) => {
            match read_package_meta_toml(&project_dir) {
                Ok(metadata) => {
                    match build_project(&project_dir) {
                        Ok(artifacts) => {
                            // We need to get the target directory to drop the product in.
                            let target_dir = if let Some(target_dir) = &arguments.target_dir {
                                target_dir.clone()
                            } else {
                                project_dir.join("target")
                            };

                            if let Err(error) = fs::create_dir_all(&target_dir) {
                                println!("{} Failed to create target directory: {}", "Error:".red(), error);
                            } else {
                                match fs::File::create(target_dir.join(&metadata.name).with_extension("zip")) {
                                    Ok(file) => {
                                        fn trampoline(
                                            file: fs::File, metadata: &PackageMetadata, artifacts: &[PathBuf],
                                        ) -> Result<(), Box<dyn std::error::Error>> {
                                            let wasm_dir = PathBuf::from("wasm");
                                            let mut zip = zip::ZipWriter::new(file);
                                            let options = zip::write::FileOptions::default()
                                                .compression_method(zip::CompressionMethod::Bzip2);

                                            // Pack in metadata
                                            let metadata = postcard::to_stdvec(metadata)?;
                                            zip.start_file("META", options)?;
                                            zip.write_all(&metadata)?;

                                            println!("Adding binary artifacts.");
                                            for artifact in artifacts {
                                                println!("{}", artifact.to_string_lossy().green());
                                                let mut file = fs::File::open(artifact)?;
                                                zip.start_file(
                                                    wasm_dir
                                                        .join(artifact.file_name().expect("Artifact path without a file name."))
                                                        .to_string_lossy(),
                                                    options,
                                                )?;

                                                // Isn't Rust beautiful?
                                                std::io::copy(&mut file, &mut zip)?;
                                            }

                                            // TODO pack in resource

                                            // Finish off the zip.
                                            zip.finish()?;

                                            Ok(())
                                        }

                                        if let Err(error) = trampoline(file, &metadata, &artifacts) {
                                            println!("{} {}", "Error while writing to mod file:".red(), error);
                                        }
                                    }
                                    Err(error) => {
                                        println!("{} {}", "Error opening mod file for writing:".red(), error);
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            println!("{} {}", "Error:".red(), error);
                        }
                    }
                }
                Err(error) => {
                    println!("{} {}", "Error reading package metadata: ".red(), error);
                }
            }
        }
        Err(error) => {
            println!("{} {}", "Error determining project directory:".red(), error);
        }
    }
}

/// Builds a whole project and then returns a list of artifacts.
fn build_project(project_dir: &PathBuf) -> Result<Vec<PathBuf>, String> {
    let projects = read_workspace_toml(&project_dir)?;

    let mut artifacts = Vec::new();

    for project in projects {
        // Try to build it with cargo.

        let project_path = project_dir.join(project);

        match build_artifact(&project_path, &[]) {
            Ok(artifact) => {
                artifacts.push(artifact);
            }
            Err(error) => {
                return Err(format!("Failed to build a project: {}", error));
            }
        }
    }

    Ok(artifacts)
}

#[derive(Serialize)]
struct PackageMetadata {
    revision: u16,
    name: String,
}

/// Reads metadata about the project.
fn read_package_meta_toml(project_dir: &PathBuf) -> Result<PackageMetadata, String> {
    let toml_file = project_dir.join("GridPackage.toml");

    if toml_file.exists() {
        let toml_file = fs::read_to_string(toml_file);

        match toml_file {
            Ok(toml_file) => {
                let toml_file = toml_file.parse::<toml::Value>();
                match toml_file {
                    Ok(toml_file) => read_package_metadata(&toml_file),
                    Err(error) => Err(format!("Failed to parse toml file: {}", error)),
                }
            }
            Err(error) => Err(format!("Failed to open toml file: {}", error)),
        }
    } else {
        Err(format!("GridPackage.toml does not exist at project root."))
    }
}

/// Gets us a list of all the projects that are part of the workspace.
fn read_workspace_toml(project_dir: &PathBuf) -> Result<Vec<String>, String> {
    // We read the config from the same Cargo.toml that cargo uses.
    // Why? Because we're actually using cargo to build all of this.
    let toml_file = project_dir.join("Cargo.toml");

    if toml_file.exists() {
        let toml_file = fs::read_to_string(toml_file);

        match toml_file {
            Ok(toml_file) => {
                let toml_file = toml_file.parse::<toml::Value>();
                match toml_file {
                    Ok(toml_file) => read_workspace_metadata(&toml_file),
                    Err(error) => Err(format!("Failed to parse toml file: {}", error)),
                }
            }
            Err(error) => Err(format!("Failed to open toml file: {}", error)),
        }
    } else {
        Err(format!("Cargo.toml does not exist at project root."))
    }
}

fn read_package_metadata(toml_file: &toml::Value) -> Result<PackageMetadata, String> {
    if let Some(package) = toml_file.get("package") {
        if let Some(name) = package.get("name") {
            match name {
                toml::Value::String(name) => Ok(PackageMetadata { revision: META_HEADER_VERSION, name: name.clone() }),
                _ => Err(format!("Module name must be specified as a string")),
            }
        } else {
            Err(format!("Module name was not provided in Cargo.toml"))
        }
    } else {
        Err(format!("Could not find package section in GridPackage.toml"))
    }
}

fn read_workspace_metadata(toml_file: &toml::Value) -> Result<Vec<String>, String> {
    if let Some(workspace) = &toml_file.get("workspace") {
        if let Some(members) = &workspace.get("members") {
            if let toml::Value::Array(members) = members {
                let mut paths = Vec::new();

                for member in members {
                    if let toml::Value::String(path) = member {
                        paths.push(path.to_string());
                    } else {
                        return Err(format!("Member project paths must all be specified as strings. Got: {}", member));
                    }
                }

                Ok(paths)
            } else {
                // There are so many ways to fail here.
                Err(format!("Member projects list in workspace Cargo.toml must be a list of strings."))
            }
        } else {
            Err(format!("Member projects have not been specified in workspace Cargo.toml."))
        }
    } else {
        Err(format!("Project root is not a cargo workspace."))
    }
}

/// Just gets the path.
/// Will fail if the path ether does not exist or is not a directory.
fn get_project_dir(arg_path: Option<PathBuf>) -> Result<PathBuf, &'static str> {
    if let Some(path) = arg_path {
        if path.exists() {
            if path.is_dir() {
                Ok(path)
            } else {
                Err("Provided path is not a directory.")
            }
        } else {
            Err("Provided path does not exist.")
        }
    } else {
        let path = std::env::current_dir();

        if let Ok(path) = path {
            Ok(path)
        } else {
            // This is a real weird case.
            Err("Failed to get current working directory.")
        }
    }
}
