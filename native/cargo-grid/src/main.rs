// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use argh::FromArgs;
use colored::*;
use common::modules::PackageMetadata;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use cargo_metadata::Message;
use std::process::{Command, Output, Stdio};

const META_HEADER_VERSION: u16 = 0;

#[derive(FromArgs, PartialEq, Debug)]
/// This is a command line tool meant to assist in the creation of content for
/// the Grid Engine.
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
/// Package your code and assets into a package that can be loaded by the grid
/// engine.
struct Pack {
    #[argh(option)]
    /// optionally specify the path of the cargo project you wish to pack. If
    /// not specified, assumes the current directory.
    path: Option<PathBuf>,

    #[argh(option)]
    /// optionally specify the path to drop the mod file into. If not specified,
    /// will default to the workspace's target directory.
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
                                            let metadata = bincode::serialize(metadata)?;
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
fn build_project(project_dir: &Path) -> Result<Vec<PathBuf>, String> {
    fn get_output(project_dir: &Path) -> Result<Output, String> {
        let project_dir = project_dir.canonicalize();
        // Yes, we just manually call cargo and then parse its output.
        let cargo_executable = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());

        match project_dir {
            Ok(project_dir) => {
                let cargo_command = Command::new(cargo_executable)
                    .current_dir(project_dir)
                    .arg("build")
                    .arg("--release") // TODO give the user a way to provide arguments, like features.
                    .args(&["--message-format", "json"])
                    .stdout(Stdio::piped())
                    .spawn();

                match cargo_command {
                    Ok(cargo_command) => {
                        // This should print all the output
                        let output = cargo_command.wait_with_output();

                        match output {
                            Ok(output) => Ok(output),
                            Err(error) => Err(format!("Failed to grab output of cargo: {}", error)),
                        }
                    }
                    Err(error) => Err(format!("Failed to launch cargo: {}", error)),
                }
            }
            Err(error) => Err(format!("Failed to get project directory: {}", error)),
        }
    }

    let output = get_output(project_dir)?;

    // Okay, the build is done. Now we parse the output to figure out what the build
    // artifacts are.
    let messages = Message::parse_stream(&output.stdout[..]);

    let mut artifacts = Vec::new();

    for message in messages {
        match message {
            Ok(message) => {
                match message {
                    Message::CompilerArtifact(artifact) => {
                        let files = &artifact.filenames;

                        for file in files {
                            // Only accept wasm artifacts.
                            if let Some(extension) = file.extension() {
                                if extension == "wasm" {
                                    artifacts.push(file.clone());
                                }
                            }
                        }
                    }
                    Message::CompilerMessage(message) => {
                        if let Some(rendered) = message.message.rendered {
                            print!("{}", rendered);
                        }
                    }
                    // Ignore other messages.
                    _ => (),
                }
            }
            Err(error) => {
                // We bail out if we fail here.
                return Err(format!("Cargo output pipe has failed: {}", error));
            }
        }
    }

    // Check for failure.
    if output.status.success() {
        Ok(artifacts)
    } else {
        Err(format!("Cargo returned exit code: {:?}", output.status.code()))
    }
}

/// Reads metadata about the project.
fn read_package_meta_toml(project_dir: &Path) -> Result<PackageMetadata, String> {
    // TODO should probably read this using serde.
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
