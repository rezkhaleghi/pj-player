#!/bin/bash

# Update Homebrew
echo "Updating Homebrew..."
brew update

# Install dependencies
echo "Installing dependencies..."
brew install ffmpeg
brew install yt-dlp

echo "All dependencies installed successfully!"