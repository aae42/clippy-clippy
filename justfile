project_name := "clippy-clippy"

# Target directory used by cargo/cross
target_dir := "target"

# Directory to place final built artifacts
artifact_dir := "artifacts"

# this list
_default:
    @just --list --unsorted

build_recipe := if os() == "macos" {
    "build-macos-arm"
} else if os() == "linux" {
    "build-windows-gnu"
} else { "" }

# Build (for macos and wsl only 🤷😄)
build:
    @just {{ build_recipe }}

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
        echo "   Install it using: `just install-cross`"; \
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

macos_arm_target := "aarch64-apple-darwin"

# Build for macOS ARM (Apple Silicon) using cross
build-macos-arm: _check-tools
    @echo "🍎 Building for macOS ARM ({{macos_arm_target}})..."
    @cross build --release --target {{macos_arm_target}} --verbose
    @mkdir -p {{artifact_dir}}/macos-arm
    @cp "{{target_dir}}/{{macos_arm_target}}/release/{{project_name}}" "{{artifact_dir}}/macos-arm/{{project_name}}"
    @echo "✅ macOS ARM build complete: {{artifact_dir}}/macos-arm/{{project_name}}"

windows_gnu_target := "x86_64-pc-windows-gnu"

# Build for Windows x86_64 (GNU toolchain) using cross
build-windows-gnu: _check-tools
    @echo " Building for Windows x86_64 GNU ({{windows_gnu_target}})..."
    @cross build --release --target {{windows_gnu_target}} --verbose
    @mkdir -p {{artifact_dir}}/windows-x64-gnu
    @cp "{{target_dir}}/{{windows_gnu_target}}/release/{{project_name}}.exe" "{{artifact_dir}}/windows-x64-gnu/{{project_name}}.exe"
    @echo "✅ Windows x86_64 GNU build complete: {{artifact_dir}}/windows-x64-gnu/{{project_name}}.exe"
