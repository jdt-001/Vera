#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Load API credentials if available
if [ -f "$REPO_ROOT/secrets.env" ]; then
    set -a
    source "$REPO_ROOT/secrets.env"
    set +a
    echo "[init] Loaded secrets.env"
else
    echo "[init] WARNING: secrets.env not found - embedding/reranker APIs will not work"
fi

# Ensure Rust toolchain is available
if ! command -v cargo &>/dev/null; then
    echo "[init] ERROR: cargo not found. Install Rust: https://rustup.rs"
    exit 1
fi

echo "[init] Rust $(rustc --version)"

# Download vendored tree-sitter grammars (Vue, Dockerfile)
# These are vendored C sources needed because their crates.io packages
# are incompatible with tree-sitter 0.26
VUE_DIR="$REPO_ROOT/crates/tree-sitter-vue/src"
if [ ! -f "$VUE_DIR/parser.c" ]; then
    echo "[init] Downloading tree-sitter-vue grammar..."
    mkdir -p "$VUE_DIR/tree_sitter"
    curl -sL "https://raw.githubusercontent.com/tree-sitter-grammars/tree-sitter-vue/main/src/parser.c" -o "$VUE_DIR/parser.c"
    curl -sL "https://raw.githubusercontent.com/tree-sitter-grammars/tree-sitter-vue/main/src/scanner.c" -o "$VUE_DIR/scanner.c"
    curl -sL "https://raw.githubusercontent.com/tree-sitter-grammars/tree-sitter-vue/main/src/tag.h" -o "$VUE_DIR/tag.h"
    curl -sL "https://raw.githubusercontent.com/tree-sitter-grammars/tree-sitter-vue/main/src/tree_sitter/parser.h" -o "$VUE_DIR/tree_sitter/parser.h"
    curl -sL "https://raw.githubusercontent.com/tree-sitter-grammars/tree-sitter-vue/main/src/tree_sitter/alloc.h" -o "$VUE_DIR/tree_sitter/alloc.h"
    curl -sL "https://raw.githubusercontent.com/tree-sitter-grammars/tree-sitter-vue/main/src/tree_sitter/array.h" -o "$VUE_DIR/tree_sitter/array.h"
    echo "[init] tree-sitter-vue grammar downloaded"
fi

DOCKERFILE_DIR="$REPO_ROOT/crates/tree-sitter-dockerfile/src"
if [ ! -f "$DOCKERFILE_DIR/parser.c" ]; then
    echo "[init] Downloading tree-sitter-dockerfile grammar..."
    mkdir -p "$DOCKERFILE_DIR/tree_sitter"
    curl -sL "https://raw.githubusercontent.com/camdencheek/tree-sitter-dockerfile/main/src/parser.c" -o "$DOCKERFILE_DIR/parser.c"
    curl -sL "https://raw.githubusercontent.com/camdencheek/tree-sitter-dockerfile/main/src/scanner.c" -o "$DOCKERFILE_DIR/scanner.c"
    curl -sL "https://raw.githubusercontent.com/camdencheek/tree-sitter-dockerfile/main/src/tree_sitter/parser.h" -o "$DOCKERFILE_DIR/tree_sitter/parser.h"
    echo "[init] tree-sitter-dockerfile grammar downloaded"
fi

# Build the project
if [ -f "$REPO_ROOT/Cargo.toml" ]; then
    echo "[init] Building project..."
    cargo build 2>&1 | tail -5
    echo "[init] Build complete"
fi

# Create benchmark repos directory
mkdir -p "$REPO_ROOT/.bench/repos"

echo "[init] Environment ready"
