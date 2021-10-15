use ansi_term::Color::{Cyan, Green, Purple, Red, Yellow};
use anyhow::Error;
use massh::{MasshClient, MasshConfig};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    cmd: Command,
    /// Path of JSON configuration file (only 1 format must be specified)
    #[structopt(short, long, conflicts_with("yaml"), required_unless("yaml"))]
    json: Option<PathBuf>,
    /// Path of YAML configuration file (only 1 format must be specified)
    #[structopt(short, long, conflicts_with("json"), required_unless("json"))]
    yaml: Option<PathBuf>,
}

#[derive(StructOpt)]
enum Command {
    /// Executes a command on the configured hosts
    Execute {
        /// Command to be executed over SSH
        command: String,
    },
    /// Downloads a file from the configured hosts
    ScpDownload {
        /// Path of download's source file on remote machine
        remote_path: PathBuf,
        /// Path of download's destination directory on local machine
        local_path: PathBuf,
    },
    /// Uploads a file to the configured hosts
    ScpUpload {
        /// Path of upload's source file on local machine
        local_path: PathBuf,
        /// Path of upload's destination file on remote machine
        remote_path: PathBuf,
    },
}

/// Configuration file formats supported by the `MasshClient` struct.
enum Format {
    Json,
    Yaml,
}

fn main() {
    // Build an `Opt` struct from the command line arguments.
    // Print an error message and exit the program on failure.
    let opt = Opt::from_args();

    // Extract the configuration file's path and format from the `Opt` struct.
    let (path, format) = if let Some(path) = opt.json {
        (path, Format::Json)
    } else if let Some(path) = opt.yaml {
        (path, Format::Yaml)
    } else {
        unreachable!();
    };

    // Build a `MasshClient` struct from the configuration file.
    // Print an error message and exit the program on failure.
    let string = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        let message = Red.paint(format!("Failed to read {:?}: {}", path, error));
        eprintln!("{}", message);
        std::process::exit(1);
    });
    let result = match format {
        Format::Json => MasshConfig::from_json(&string),
        Format::Yaml => MasshConfig::from_yaml(&string),
    };
    let config = result.unwrap_or_else(|error| {
        let message = Red.paint(format!("Failed to parse {:?}: {}", path, error));
        eprintln!("{}", message);
        std::process::exit(1);
    });
    let massh = MasshClient::from(&config);

    // Match the subcommand and call the corresponding `MasshClient` method, all of which return
    // the receiving half of a `std::sync::mpsc::channel`. Exactly 1 message per host is received.
    let (mut num_success, mut num_warning, mut num_failure) = (0, 0, 0);
    match &opt.cmd {
        // Process the `execute` subcommand's received messages.
        Command::Execute { command } => {
            let rx = massh.execute(command);
            while let Ok((host, result)) = rx.recv() {
                match result {
                    Ok(output) => {
                        if output.exit_status == 0 {
                            // Print green message if result is ok and exit status is zero.
                            print_success(host, &mut num_success);
                        } else {
                            // Print yellow message if result is ok and exit status is nonzero.
                            print_warning(host, &mut num_warning, output.exit_status);
                        }
                        // Print standard output in cyan and standard error in purple.
                        print_bytes(&output.stdout, true);
                        print_bytes(&output.stderr, false);
                    }
                    // Print red message if result is not ok.
                    Err(error) => print_failure(host, &mut num_failure, error),
                }
            }
        }
        // Process the `scp-download` and `scp-upload` subcommands' received messages.
        _ => {
            let rx = match &opt.cmd {
                Command::ScpDownload {
                    remote_path,
                    local_path,
                } => massh.scp_download(remote_path, local_path),
                Command::ScpUpload {
                    local_path,
                    remote_path,
                } => massh.scp_upload(local_path, remote_path),
                _ => unreachable!(),
            };
            while let Ok((host, result)) = rx.recv() {
                match result {
                    // Print green message if result is ok.
                    Ok(()) => print_success(host, &mut num_success),
                    // Print red message if result is not ok.
                    Err(error) => print_failure(host, &mut num_failure, error),
                }
            }
        }
    }

    // Print summaries of the number of successes, warnings, and failures.
    println!();
    print_summary("success", num_success);
    print_summary("warning", num_warning);
    print_summary("failure", num_failure);
}

/// Prints a summary of the number of successes, warnings, or failures.
fn print_summary(label: &str, count: usize) {
    let color = match label {
        "success" => Green,
        "warning" => Yellow,
        "failure" => Red,
        _ => unreachable!(),
    };
    let noun = if count == 1 { "host" } else { "hosts" };
    let message = format!("{}: {} {}", label, count, noun);
    println!("{}", color.paint(message));
}

/// Prints host's success message in green.
fn print_success(host: String, count: &mut usize) {
    *count += 1;
    let message = Green.paint("success");
    println!("[{}]: {}", host, message);
}

/// Prints host's warning message in yellow.
fn print_warning(host: String, count: &mut usize, exit_status: i32) {
    *count += 1;
    let message = Yellow.paint(format!("warning: exit status = {}", exit_status));
    println!("[{}]: {}", host, message);
}

/// Prints host's failure message in red.
fn print_failure(host: String, count: &mut usize, error: Error) {
    *count += 1;
    let message = Red.paint(format!("failure: {}", error));
    println!("[{}]: {}", host, message);
}

/// Prints standard output in cyan or standard error in purple.
fn print_bytes(bytes: &[u8], stdout: bool) {
    if !bytes.is_empty() {
        let color = if stdout { Cyan } else { Purple };
        let label = if stdout { "stdout" } else { "stderr" };
        if let Ok(message) = std::str::from_utf8(bytes) {
            println!("{}", color.paint(message.trim_end()));
        } else {
            let message = format!("{} is not UTF-8 ({} bytes)", label, bytes.len());
            println!("{}", color.paint(message));
        }
    }
}
