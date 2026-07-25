#!/usr/bin/env bash
set -euo pipefail

if ! command -v ollama >/dev/null 2>&1; then
  echo "Installing Ollama..."
  curl -fsSL https://ollama.com/install.sh | sh
else
  echo "Ollama already installed"
fi

echo "Pulling recommended models..."
ollama pull qwen3.5:9b
ollama pull qwen3.5:4b
ollama pull nomic-embed-text

echo "Done. Run: atsassin profile init --resume <your-resume>"
