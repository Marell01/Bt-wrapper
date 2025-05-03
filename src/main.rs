//! Bluetooth CLI Wrapper
//! A simple CLI tool that wraps system bluetoothctl commands

use std::process::Command;
use std::str;
use std::thread;
use std::time::Duration;
use std::io::{self, Write};
use clap::{Parser, Subcommand};
use console::{style, Term};
use text_io::read;

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
        Some(Commands::Scan { duration }) => scan_devices((*duration).into()),
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

/// Enhanced scan for Bluetooth devices with separate paired and discovered device lists
fn enhanced_scan_devices(duration_seconds: u64) {
    // Get list of already paired devices before scanning
    let paired_devices = get_available_devices();
    let paired_addresses: Vec<String> = paired_devices.iter()
        .map(|(addr, _)| addr.clone())
        .collect();
    
    // Start scan
    match run_bluetoothctl_interactive("scan on") {
        Ok(_) => {
            // Visual feedback that scanning is in progress
            print!("Scanning");
            for _ in 0..duration_seconds {
                print!(".");
                io::stdout().flush().unwrap();
                thread::sleep(Duration::from_secs(1));
            }
            println!();
            
            // Stop scan
            match run_bluetoothctl_interactive("scan off") {
                Ok(_) => {
                    // Get devices after scanning
                    match run_bluetoothctl(&["devices"]) {
                        Ok(output) => {
                            let mut discovered_devices: Vec<(String, String)> = Vec::new();
                            
                            // Process and categorize all devices
                            for line in output.lines() {
                                if line.starts_with("Device") {
                                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                                    if parts.len() >= 3 {
                                        let addr = parts[1].to_string();
                                        let name = parts[2].to_string();
                                        
                                        // If not in paired_addresses, it's newly discovered
                                        if !paired_addresses.contains(&addr) {
                                            discovered_devices.push((addr, name));
                                        }
                                    }
                                }
                            }
                            
                            // Display paired devices
                            println!();
                            println!("{}", style("Paired Devices:").bold().green());
                            println!("{}", style("----------------------------").green());
                            if paired_devices.is_empty() {
                                println!("  {}", style("No paired devices").dim());
                            } else {
                                for (i, (addr, name)) in paired_devices.iter().enumerate() {
                                    println!("  {}. {} ({})", i+1, style(name).green(), addr);
                                }
                            }
                            
                            // Display newly discovered devices
                            println!();
                            println!("{}", style("Newly Discovered Devices:").bold().cyan());
                            println!("{}", style("----------------------------").cyan());
                            if discovered_devices.is_empty() {
                                println!("  {}", style("No new devices discovered").dim());
                            } else {
                                for (i, (addr, name)) in discovered_devices.iter().enumerate() {
                                    println!("  {}. {} ({})", i+1, style(name).cyan(), addr);
                                }
                            }
                            
                            // Offer connection option for newly discovered devices
                            if !discovered_devices.is_empty() {
                                println!();
                                println!("Would you like to pair with a newly discovered device? (y/n)");
                                print!("Choice: ");
                                io::stdout().flush().unwrap();
                                
                                let term = Term::stdout();
                                let choice = term.read_char().unwrap_or('n');
                                
                                if choice == 'y' || choice == 'Y' {
                                    println!();
                                    println!("Select a device to pair with:");
                                    for (i, (_, name)) in discovered_devices.iter().enumerate() {
                                        println!("  {}. {}", i+1, name);
                                    }
                                    println!("  q. Cancel");
                                    
                                    print!("Choice: ");
                                    io::stdout().flush().unwrap();
                                    let dev_choice = term.read_char().unwrap_or('q');
                                    
                                    if dev_choice == 'q' || dev_choice == 'Q' {
                                        return;
                                    }
                                    
                                    if let Some(idx) = dev_choice.to_digit(10) {
                                        let idx = idx as usize;
                                        if idx > 0 && idx <= discovered_devices.len() {
                                            let (addr, name) = &discovered_devices[idx-1];
                                            println!("\nPairing with {}...", name);
                                            pair_device(addr);
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

/// Scan for Bluetooth devices (basic version)
fn scan_devices(duration_seconds: u64) {
    println!("Scanning for Bluetooth devices for {} seconds...", duration_seconds);
    
    // Start scan
    match run_bluetoothctl_interactive("scan on") {
        Ok(_) => {
            // Sleep for the specified duration
            thread::sleep(Duration::from_secs(duration_seconds));
            
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

/// Terminate Bluetooth adapter at hardware level using rfkill
fn terminate_bluetooth_hardware() {
    let output = Command::new("rfkill")
        .args(&["block", "bluetooth"])
        .output();
        
    match output {
        Ok(o) => {
            if o.status.success() {
                println!("Bluetooth adapter blocked at hardware level.");
            } else {
                println!("Failed to block Bluetooth: {}", String::from_utf8_lossy(&o.stderr));
            }
        },
        Err(e) => println!("Error running rfkill: {}", e),
    }
}

/// Start Bluetooth adapter at hardware level using rfkill
fn start_bluetooth_hardware() {
    let output = Command::new("rfkill")
        .args(&["unblock", "bluetooth"])
        .output();
        
    match output {
        Ok(o) => {
            if o.status.success() {
                println!("Bluetooth adapter unblocked at hardware level.");
                // Also try to power on using bluetoothctl
                power_on();
            } else {
                println!("Failed to unblock Bluetooth: {}", String::from_utf8_lossy(&o.stderr));
            }
        },
        Err(e) => println!("Error running rfkill: {}", e),
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

/// Unblock Bluetooth at hardware level - runs on startup
fn ensure_bluetooth_unblocked() {
    // Run rfkill to ensure Bluetooth is unblocked after system restart
    let _ = Command::new("rfkill")
        .args(&["unblock", "bluetooth"])
        .output(); // We don't check the result - just try it
}

/// Run interactive Bluetooth menu
fn interactive_menu() {
    let term = Term::stdout();
    
    // Ensure Bluetooth is unblocked at hardware level at startup
    ensure_bluetooth_unblocked();
    
    loop {
        // Clear the terminal
        let _ = term.clear_screen();
        
        println!("{}", style("Bluetooth Manager").bold().cyan());
        println!("{}", style("================").cyan());
        
        // Check Bluetooth hardware status
        let power_status = is_bluetooth_powered();
        let power_text = if power_status {
            style("ON").green().bold()
        } else {
            style("OFF").red().bold()
        };
        
        println!("{} {}", style("Bluetooth Hardware Status:").bold(), power_text);
        
        // Only show connected devices if Bluetooth is powered on
        if power_status {
            // Get connected devices
            let connected_devices = get_connected_devices();
            
            println!();
            if connected_devices.is_empty() {
                println!("{}", style("No devices connected").yellow());
            } else {
                println!("{}", style("Connected Devices:").bold());
                for (index, (address, name)) in connected_devices.iter().enumerate() {
                    println!("  {}. {} ({})", index + 1, style(name).green(), address);
                }
            }
            
            // Show menu options based on connection state
            println!();
            println!("{}", style("Options:").bold());
            
            if power_status {
                println!("  1. {}", style("Turn Bluetooth OFF").red());
                
                if !connected_devices.is_empty() {
                    println!("  2. {}", style("Disconnect from device").yellow());
                    println!("  3. {}", style("Connect to another device").green());
                } else {
                    println!("  2. {}", style("Connect to a device").green());
                }
                
                println!("  6. {}", style("Scan for devices").cyan());
                println!("  t. {}", style("Terminate Bluetooth adapter (hardware)").red().bold());
                println!("  r. {}", style("Restart Bluetooth adapter (hardware)").yellow());
            } else {
                println!("  1. {}", style("Turn Bluetooth ON").green());
                println!("  s. {}", style("Start Bluetooth adapter (hardware)").green().bold());
            }
            
            println!("  q. {}", style("Quit").red());
            
            // Get user choice without waiting for Enter
            print!("\nEnter your choice: ");
            io::stdout().flush().unwrap();
            let choice = term.read_char().unwrap_or('x');
            
            match choice {
                '1' => {
                    if power_status {
                        power_off();
                        println!("Turned Bluetooth OFF. Press Enter to continue...");
                    } else {
                        power_on();
                        println!("Turned Bluetooth ON. Press Enter to continue...");
                    }
                    wait_for_enter();
                },
                '2' => {
                    if power_status {
                        if !connected_devices.is_empty() {
                            // Disconnect option
                            if connected_devices.len() == 1 {
                                // Only one device, disconnect directly
                                let (address, name) = &connected_devices[0];
                                println!("Disconnecting from {}...", name);
                                disconnect_device(address);
                            } else {
                                // Multiple devices, ask which one to disconnect
                                println!("\nSelect a device to disconnect from:");
                                for (index, (_, name)) in connected_devices.iter().enumerate() {
                                    println!("  {}. {}", index + 1, name);
                                }
                                
                                print!("Enter device number: ");
                                io::stdout().flush().unwrap();
                                let device_choice: String = read!("{}");
                                
                                if let Ok(idx) = device_choice.trim().parse::<usize>() {
                                    if idx > 0 && idx <= connected_devices.len() {
                                        let (address, name) = &connected_devices[idx - 1];
                                        println!("Disconnecting from {}...", name);
                                        disconnect_device(address);
                                    }
                                }
                            }
                        } else {
                            // Connect option (when no devices are connected)
                            connect_to_device_menu();
                        }
                        wait_for_enter();
                    }
                },
                '3' => {
                    if power_status && !connected_devices.is_empty() {
                        connect_to_device_menu();
                        wait_for_enter();
                    }
                },
                '6' => {
                    if power_status {
                        // Clear screen and show enhanced scanning interface
                        let _ = term.clear_screen();
                        println!("{}", style("Scanning for Bluetooth Devices").bold().cyan());
                        println!("{}", style("===========================").cyan());
                        println!("Scanning for 8 seconds...");
                        
                        // Enhanced scanning with separate menus for paired and new devices
                        enhanced_scan_devices(8);
                        
                        println!("\nPress Enter to return to main menu...");
                        wait_for_enter();
                    }
                },
                't' => {
                    if power_status {
                        // Block Bluetooth at hardware level
                        terminate_bluetooth_hardware();
                        println!("Bluetooth adapter terminated at hardware level. Press Enter to continue...");
                        wait_for_enter();
                    }
                },
                's' | 'S' => {
                    if !power_status {
                        // Unblock Bluetooth at hardware level
                        start_bluetooth_hardware();
                        println!("Bluetooth adapter started at hardware level. Press Enter to continue...");
                        wait_for_enter();
                    }
                },
                'r' => {
                    if power_status {
                        // Restart Bluetooth at hardware level
                        println!("Restarting Bluetooth adapter at hardware level...");
                        terminate_bluetooth_hardware();
                        thread::sleep(Duration::from_secs(1));
                        start_bluetooth_hardware();
                        println!("Bluetooth adapter restarted. Press Enter to continue...");
                        wait_for_enter();
                    }
                },
                'q' | 'Q' => break,
                _ => {
                    println!("Invalid option. Press Enter to continue...");
                    wait_for_enter();
                }
            }
        } else {
            // Bluetooth is off, show limited menu
            println!();
            println!("{}", style("Options:").bold());
            println!("  1. {}", style("Turn Bluetooth ON").green());
            println!("  q. {}", style("Quit").red());
            
            // Get user choice without waiting for Enter
            print!("\nEnter your choice: ");
            io::stdout().flush().unwrap();
            let choice = term.read_char().unwrap_or('x');
            
            match choice {
                '1' => {
                    power_on();
                    println!("Turned Bluetooth ON. Press Enter to continue...");
                    wait_for_enter();
                },
                'q' | 'Q' => break,
                _ => {
                    println!("Invalid option. Press Enter to continue...");
                    wait_for_enter();
                }
            }
        }
    }
}

/// Present a menu to connect to a device
fn connect_to_device_menu() {
    let term = Term::stdout();
    
    // Get available devices
    let available_devices = get_available_devices();
    
    if available_devices.is_empty() {
        println!("No paired devices available. Scan first?");
        return;
    }
    
    println!("\nSelect a device to connect to:");
    for (index, (_, name)) in available_devices.iter().enumerate() {
        println!("  {}. {}", index + 1, name);
    }
    println!("  q. {}", style("Cancel").yellow());
    
    print!("Enter choice: ");
    io::stdout().flush().unwrap();
    let choice = term.read_char().unwrap_or('x');
    
    if choice == 'q' || choice == 'Q' {
        println!("\nCanceled.");
        return;
    }
    
    // Convert char to number (1-9)
    if let Some(digit) = choice.to_digit(10) {
        let idx = digit as usize;
        if idx > 0 && idx <= available_devices.len() {
            let (address, name) = &available_devices[idx - 1];
            println!("Connecting to {}...", name);
            connect_device(address);
        } else {
            println!("Invalid selection.");
        }
    } else {
        println!("Invalid selection.");
    }
}

/// Wait for the user to press Enter
fn wait_for_enter() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

/// Check if Bluetooth is powered on
fn is_bluetooth_powered() -> bool {
    match run_bluetoothctl(&["show"]) {
        Ok(output) => output.contains("Powered: yes"),
        Err(_) => false,
    }
}

/// Get a list of connected devices
fn get_connected_devices() -> Vec<(String, String)> {
    let mut connected_devices = Vec::new();
    
    match run_bluetoothctl(&["devices"]) {
        Ok(output) => {
            for line in output.lines() {
                if line.starts_with("Device") {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        // Check if this device is connected
                        if let Ok(device_info) = run_bluetoothctl(&["info", parts[1]]) {
                            if device_info.contains("Connected: yes") {
                                connected_devices.push((parts[1].to_string(), parts[2].to_string()));
                            }
                        }
                    }
                }
            }
        },
        Err(_) => {},
    }
    
    connected_devices
}

/// Get a list of all available (paired) devices
fn get_available_devices() -> Vec<(String, String)> {
    let mut available_devices = Vec::new();
    
    match run_bluetoothctl(&["devices"]) {
        Ok(output) => {
            for line in output.lines() {
                if line.starts_with("Device") {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        available_devices.push((parts[1].to_string(), parts[2].to_string()));
                    }
                }
            }
        },
        Err(_) => {},
    }
    
    available_devices
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