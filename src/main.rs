//! Bluetooth CLI Wrapper
//! A simple CLI tool that wraps system bluetoothctl commands

use std::process::Command;
use std::str;
use std::thread;
use std::time::Duration;
use clap::{Parser, Subcommand};

/// Bluetooth command-line wrapper
#[derive(Parser)]
#[command(name = "bt")]
#[command(about = "Bluetooth command line utility", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Available Bluetooth commands
#[derive(Subcommand)]
enum Commands {
    /// Display version information
    #[command(alias = "v")]
    Version,
    
    /// List paired devices
    #[command(alias = "devices")]
    List,
    
    /// Connect to a device by MAC address
    Connect {
        #[arg(help = "Device MAC address (format: XX:XX:XX:XX:XX:XX)")]
        address: String,
    },
    
    /// Disconnect from a device by MAC address
    Disconnect {
        #[arg(help = "Device MAC address (format: XX:XX:XX:XX:XX:XX)")]
        address: String,
    },
    
    /// Scan for new devices
    Scan {
        /// Duration in seconds to scan
        #[arg(short, long, default_value_t = 5)]
        duration: u8,
    },
    
    /// Turn Bluetooth on
    #[command(alias = "on")]
    Power,
    
    /// Turn Bluetooth off
    #[command(alias = "off")]
    Poweroff,

    /// Show adapter info
    Info,

    /// Pair with a device
    Pair {
        #[arg(help = "Device MAC address (format: XX:XX:XX:XX:XX:XX)")]
        address: String,
    },

    /// Remove a paired device
    Remove {
        #[arg(help = "Device MAC address (format: XX:XX:XX:XX:XX:XX)")]
        address: String,
    },
}

fn main() {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Execute the requested command
    match &cli.command {
        Some(Commands::Version) => show_version(),
        Some(Commands::List) => list_devices(),
        Some(Commands::Connect { address }) => connect_device(address),
        Some(Commands::Disconnect { address }) => disconnect_device(address),
        Some(Commands::Scan { duration }) => scan_devices(*duration),
        Some(Commands::Power) => power_on(),
        Some(Commands::Poweroff) => power_off(),
        Some(Commands::Info) => show_info(),
        Some(Commands::Pair { address }) => pair_device(address),
        Some(Commands::Remove { address }) => remove_device(address),
        None => interactive_menu(), // Default when no subcommand is provided
    }
}

/// Run a bluetoothctl command and return its output
fn run_bluetoothctl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("bluetoothctl")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute bluetoothctl: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "Command failed: {} (error: {})",
            String::from_utf8_lossy(&output.stderr).trim(),
            output.status
        ))
    }
}

/// Run a command that expects interactive input by using echo and pipe
fn run_bluetoothctl_interactive(command: &str) -> Result<String, String> {
    // Creates a command like: echo "command" | bluetoothctl
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("echo '{}' | bluetoothctl", command))
        .output()
        .map_err(|e| format!("Failed to execute bluetoothctl: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "Command failed: {} (error: {})",
            String::from_utf8_lossy(&output.stderr).trim(),
            output.status
        ))
    }
}

/// List paired devices
fn list_devices() {
    println!("Paired Bluetooth devices:\n");
    
    match run_bluetoothctl(&["devices"]) {
        Ok(output) => {
            if output.is_empty() {
                println!("No paired devices found.");
                return;
            }
            
            // Parse and format the output for better display
            for line in output.lines() {
                if line.starts_with("Device") {
                    // Format: Device XX:XX:XX:XX:XX:XX DeviceName
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        println!("  {} - {}", parts[1], parts[2]);
                    } else {
                        println!("  {}", parts[1]);
                    }
                }
            }
        },
        Err(err) => println!("Error listing devices: {}", err),
    }
}

/// Connect to a Bluetooth device
fn connect_device(address: &str) {
    println!("Connecting to device {}...", address);
    
    match run_bluetoothctl(&["connect", address]) {
        Ok(output) => {
            if output.contains("successful") || output.contains("Connection successful") {
                println!("Successfully connected to device {}", address);
            } else {
                println!("Connection attempt completed, but success unclear.\nOutput: {}", output);
            }
        },
        Err(err) => println!("Error connecting to device: {}", err),
    }
}

/// Disconnect from a Bluetooth device
fn disconnect_device(address: &str) {
    println!("Disconnecting from device {}...", address);
    
    match run_bluetoothctl(&["disconnect", address]) {
        Ok(output) => {
            if output.contains("successful") || output.contains("Disconnection successful") {
                println!("Successfully disconnected from device {}", address);
            } else {
                println!("Disconnection attempt completed, but success unclear.\nOutput: {}", output);
            }
        },
        Err(err) => println!("Error disconnecting from device: {}", err),
    }
}

/// Scan for Bluetooth devices
fn scan_devices(duration: u8) {
    println!("Scanning for Bluetooth devices for {} seconds...", duration);
    
    // Start scan
    match run_bluetoothctl_interactive("scan on") {
        Ok(_) => {
            // Sleep for the specified duration
            thread::sleep(Duration::from_secs(duration as u64));
            
            // Stop scan
            match run_bluetoothctl_interactive("scan off") {
                Ok(_) => {
                    // List devices found
                    match run_bluetoothctl(&["devices"]) {
                        Ok(output) => {
                            println!("\nDevices found:");
                            if output.is_empty() {
                                println!("No devices found.");
                            } else {
                                for line in output.lines() {
                                    if line.starts_with("Device") {
                                        let parts: Vec<&str> = line.splitn(3, ' ').collect();
                                        if parts.len() >= 3 {
                                            println!("  {} - {}", parts[1], parts[2]);
                                        } else {
                                            println!("  {}", parts[1]);
                                        }
                                    }
                                }
                            }
                        },
                        Err(err) => println!("Error listing devices: {}", err),
                    }
                },
                Err(err) => println!("Error stopping scan: {}", err),
            }
        },
        Err(err) => println!("Error starting scan: {}", err),
    }
}

/// Turn Bluetooth on
fn power_on() {
    println!("Turning Bluetooth on...");
    
    match run_bluetoothctl(&["power", "on"]) {
        Ok(_) => println!("Bluetooth turned on successfully."),
        Err(err) => println!("Error turning Bluetooth on: {}", err),
    }
}

/// Turn Bluetooth off
fn power_off() {
    println!("Turning Bluetooth off...");
    
    match run_bluetoothctl(&["power", "off"]) {
        Ok(_) => println!("Bluetooth turned off successfully."),
        Err(err) => println!("Error turning Bluetooth off: {}", err),
    }
}

/// Show adapter information
fn show_info() {
    println!("Bluetooth adapter information:\n");
    
    match run_bluetoothctl(&["show"]) {
        Ok(output) => {
            for line in output.lines() {
                // Format the output for better readability
                if !line.trim().is_empty() {
                    println!("  {}", line.trim());
                }
            }
        },
        Err(err) => println!("Error getting adapter info: {}", err),
    }
}

/// Pair with a device
fn pair_device(address: &str) {
    println!("Pairing with device {}...", address);
    
    match run_bluetoothctl(&["pair", address]) {
        Ok(output) => {
            if output.contains("successful") || output.contains("Pairing successful") {
                println!("Successfully paired with device {}", address);
            } else {
                println!("Pairing attempt completed, but success unclear.\nOutput: {}", output);
            }
        },
        Err(err) => println!("Error pairing with device: {}", err),
    }
}

/// Remove a paired device
fn remove_device(address: &str) {
    println!("Removing paired device {}...", address);
    
    match run_bluetoothctl(&["remove", address]) {
        Ok(output) => {
            if output.contains("successful") || output.contains("Device has been removed") {
                println!("Successfully removed device {}", address);
            } else {
                println!("Remove attempt completed, but success unclear.\nOutput: {}", output);
            }
        },
        Err(err) => println!("Error removing device: {}", err),
    }
}

/// Display version information of the Bluetooth wrapper
fn show_version() {
    // Get the version from Cargo.toml via env variable
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");
    
    println!("Bluetooth Wrapper v{}", version);
    println!("Package name: {}", name);
    println!("Build date: {}", env!("CARGO_PKG_VERSION_MAJOR"));
    
    // Also display system Bluetooth information
    match run_bluetoothctl(&["version"]) {
        Ok(output) => {
            println!("\nSystem Bluetooth version:");
            println!("{}", output);
        },
        Err(_) => {
            println!("\nCould not determine system Bluetooth version.");
        },
    }
}