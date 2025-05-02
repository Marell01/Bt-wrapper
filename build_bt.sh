#!/bin/bash
# build_bt.sh - Script to build and install the Bluetooth wrapper
# This script automates building the project and installing it to ~/.local/bin

# Set colors for better output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Starting Bluetooth wrapper build process...${NC}"

# Check if the current directory has a Cargo.toml file
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Cargo.toml not found. Make sure you're in the project directory.${NC}"
    exit 1
fi

# Build the project in release mode
echo -e "${YELLOW}Building project in release mode...${NC}"
cargo build --release

# Check if build was successful
if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Build failed.${NC}"
    exit 1
fi

# Create ~/.local/bin if it doesn't exist
if [ ! -d "$HOME/.local/bin" ]; then
    echo -e "${YELLOW}Creating ~/.local/bin directory...${NC}"
    mkdir -p "$HOME/.local/bin"
fi

# Copy the executable to ~/.local/bin/bt
echo -e "${YELLOW}Installing to ~/.local/bin/bt...${NC}"
cp "target/release/bluetooth_wrapper" "$HOME/.local/bin/bt"

# Make sure it's executable
chmod +x "$HOME/.local/bin/bt"

# Check if installation was successful
if [ -x "$HOME/.local/bin/bt" ]; then
    echo -e "${GREEN}Installation successful!${NC}"
    echo -e "${YELLOW}Testing installation:${NC}"
    
    # Check version
    echo -e "${YELLOW}Version information:${NC}"
    "$HOME/.local/bin/bt" version
    
    echo -e "\n${GREEN}You can now use the 'bt' command${NC}"
else
    echo -e "${RED}Error: Installation failed.${NC}"
    exit 1
fi

exit 0
