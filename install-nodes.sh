#!/bin/bash
set -e

echo "==========================================="
echo "  MoFA Studio - Python Nodes Installer"
echo "==========================================="
echo ""
echo "This script will:"
echo "1. Clone the mofa-studio repository to ~/.mofa/studio"
echo "2. Install Pixi (if not already installed)"
echo "3. Download and configure all Python dependencies via Pixi"
echo ""

# Configuration
MOFA_DIR="$HOME/.mofa/studio"
REPO_URL="https://github.com/mofa-org/mofa-studio.git"

# 1. Ensure git is installed
if ! command -v git &> /dev/null; then
    echo "❌ Error: git is not installed. Please install git and try again."
    exit 1
fi

# 2. Clone or pull the repository
if [ -d "$MOFA_DIR" ]; then
    echo "ℹ️  Directory $MOFA_DIR already exists."
    echo "🔄 Pulling latest changes..."
    cd "$MOFA_DIR"
    git pull origin main
else
    echo "📦 Cloning mofa-studio to $MOFA_DIR..."
    mkdir -p "$HOME/.mofa"
    git clone "$REPO_URL" "$MOFA_DIR"
    cd "$MOFA_DIR"
fi

# 3. Ensure pixi is installed
if ! command -v pixi &> /dev/null; then
    echo "🔧 Pixi is not installed. Installing Pixi..."
    curl -fsSL https://pixi.sh/install.sh | bash
    # Export path for the current script execution
    export PATH="$HOME/.pixi/bin:$PATH"
    
    if ! command -v pixi &> /dev/null; then
        echo "❌ Error: Failed to install or find Pixi in PATH."
        exit 1
    fi
    echo "✅ Pixi installed successfully."
else
    echo "✅ Pixi is already installed."
fi

# 4. Install dependencies using pixi
echo "⚙️  Installing Python dependencies and configuring Dora nodes..."
echo "⏳ This may take a few minutes as it downloads PyTorch and other AI libraries..."
cd "$MOFA_DIR"
pixi install

echo ""
echo "==========================================="
echo "✅ Installation Complete!"
echo "==========================================="
echo "The Python nodes have been installed in an isolated environment at:"
echo "  $MOFA_DIR/.pixi/envs/default"
echo ""
echo "You can now launch MoFA Studio, and it will automatically"
echo "use this environment to spawn the Dora dataflows."
