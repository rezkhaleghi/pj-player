#!/bin/bash

# Update package list and install Rust
echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Install yt-dlp
echo "Installing yt-dlp..."
sudo apt-get update
sudo apt-get install -y yt-dlp

# Install wget
echo "Installing wget..."
sudo apt-get install -y wget

# Install ffmpeg
echo "Installing ffmpeg..."
sudo apt install -y ffmpeg

# Install Cava
echo "Installing Cava..."
sudo apt-get install -y cava

# Install dotenv crate
# echo "Installing dotenv crate..."
# cargo install dotenv

echo "All dependencies installed successfully."
