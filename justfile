
_default:
  @just --list --unsorted

run:
  cargo run

build:
  cargo build

# Project name (must match Cargo.toml)
project_name := "clippy-clippy"

# Target directory used by cargo/cross
target_dir := "target"

# Directory to place final built artifacts
artifact_dir := "artifacts"

# Target triples
macos_arm_target := "aarch64-apple-darwin"
windows_gnu_target := "x86_64-pc-windows-gnu" # Uses MinGW, generally easier for cross-compilation from Linux/macOS
windows_msvc_target := "x86_64-pc-windows-msvc" # Alternative: uses MSVC linker, might need more setup

# === Recipes ===

# Default recipe: build all cross-compiled targets
default: all

# Check for required tools (Rust, Cargo, Cross)
_check-tools:
    @if ! command -v rustc &> /dev/null; then \
        echo "❌ Rust is not installed. Please install it via rustup: https://rustup.rs/"; \
        exit 1; \
    fi
    @if ! command -v cargo &> /dev/null; then \
        echo "❌ Cargo is not installed. This is unexpected if Rust is installed via rustup."; \
        exit 1; \
    fi
    @if ! command -v cross &> /dev/null; then \
        echo "❌ 'cross' is not installed or not in PATH."; \
        echo "   Install it using: cargo install cross --git https://github.com/cross-rs/cross"; \
        echo "   Ensure Docker or Podman is running."; \
        exit 1; \
    fi
    @echo "✅ Required tools (rustc, cargo, cross) found."

# Install or update the cross tool
install-cross:
    @echo "⏳ Installing/updating 'cross'..."
    @cargo install cross --git https://github.com/cross-rs/cross
    @echo "✅ 'cross' installed/updated."

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    @cargo clean
    @rm -rf {{artifact_dir}}
    @echo "✅ Clean complete."

# Build for macOS ARM (Apple Silicon) using cross
build-macos-arm: _check-tools
    @echo "🍎 Building for macOS ARM ({{macos_arm_target}})..."
    @cross build --release --target {{macos_arm_target}} --verbose
    @mkdir -p {{artifact_dir}}/macos-arm
    @cp "{{target_dir}}/{{macos_arm_target}}/release/{{project_name}}" "{{artifact_dir}}/macos-arm/{{project_name}}"
    @echo "✅ macOS ARM build complete: {{artifact_dir}}/macos-arm/{{project_name}}"

# Build for Windows x86_64 (GNU toolchain) using cross
build-windows-gnu: _check-tools
    @echo " Building for Windows x86_64 GNU ({{windows_gnu_target}})..."
    @cross build --release --target {{windows_gnu_target}} --verbose
    @mkdir -p {{artifact_dir}}/windows-x64-gnu
    @cp "{{target_dir}}/{{windows_gnu_target}}/release/{{project_name}}.exe" "{{artifact_dir}}/windows-x64-gnu/{{project_name}}.exe"
    @echo "✅ Windows x86_64 GNU build complete: {{artifact_dir}}/windows-x64-gnu/{{project_name}}.exe"

# (Optional) Build for Windows x86_64 (MSVC toolchain) using cross
build-windows-msvc: _check-tools
  @echo " Building for Windows x86_64 MSVC ({{windows_msvc_target}})..."
  @cross build --release --target {{windows_msvc_target}} --verbose
  @mkdir -p {{artifact_dir}}/windows-x64-msvc
  @cp "{{target_dir}}/{{windows_msvc_target}}/release/{{project_name}}.exe" "{{artifact_dir}}/windows-x64-msvc/{{project_name}}.exe"
  @echo "✅ Windows x86_64 MSVC build complete: {{artifact_dir}}/windows-x64-msvc/{{project_name}}.exe"

# (Optional) Build natively for the host system (Linux or macOS)
build-native: _check-tools
    @echo "🖥️ Building natively for host system..."
    @cargo build --release --verbose
    @mkdir -p {{artifact_dir}}/native
    @# Determine host OS for correct artifact path/name (basic check)
    @if [[ "$(uname -s)" == "Linux" ]]; then \
        cp "{{target_dir}}/release/{{project_name}}" "{{artifact_dir}}/native/{{project_name}}"; \
        echo "✅ Native Linux build complete: {{artifact_dir}}/native/{{project_name}}"; \
    elif [[ "$(uname -s)" == "Darwin" ]]; then \
        cp "{{target_dir}}/release/{{project_name}}" "{{artifact_dir}}/native/{{project_name}}"; \
        echo "✅ Native macOS build complete: {{artifact_dir}}/native/{{project_name}}"; \
    else \
        echo "⚠️ Native build artifact path unknown for $(uname -s), check target/release/"; \
    fi

# Build all defined cross-compile targets
all: build-macos-arm build-windows-gnu # Add build-windows-msvc here if using
    @echo "🎉 All cross-compilation builds finished!"
    @echo "   Artifacts are located in the '{{artifact_dir}}' directory."