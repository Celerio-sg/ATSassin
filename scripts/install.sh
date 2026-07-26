#!/usr/bin/env bash
set -euo pipefail

echo "=== ATSassin Installer ==="
echo "Installing Rust toolchain..."
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
else
  echo "Rust already installed: $(cargo --version)"
fi

echo "Cloning ATSassin..."
if [ ! -d "ATSassin" ]; then
  git clone https://github.com/Celerio-sg/ATSassin.git
fi
cd ATSassin

echo "Building release binary..."
cargo build --release

echo "Setting up configuration..."
cp .env.example .env
echo "Edit .env with your API keys and preferences."

echo "Setting up Ollama..."
if command -v ollama >/dev/null 2>&1; then
  echo "Pulling recommended models..."
  ollama pull qwen3.5:9b
  ollama pull qwen3.5:4b
  ollama pull nomic-embed-text
else
  echo "Ollama not found. Install from https://ollama.com"
fi

echo "=== Installation complete ==="
echo "Run: ./target/release/atsassin profile init --resume <your-resume>"
