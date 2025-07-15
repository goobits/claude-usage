#!/bin/bash

set -e

echo "📦 Packaging Claude Usage - All Implementations"
echo "=============================================="

VERSION=${1:-1.0.0}
PACKAGE_DIR="claude-usage-${VERSION}"

# Create package directory
mkdir -p "dist/${PACKAGE_DIR}"

# Copy source files
echo "📂 Copying source files..."
cp -r rust/ "dist/${PACKAGE_DIR}/"
cp claude_usage.py "dist/${PACKAGE_DIR}/"
cp pyproject.toml "dist/${PACKAGE_DIR}/"
cp README.md "dist/${PACKAGE_DIR}/"
cp CLAUDE.md "dist/${PACKAGE_DIR}/"
cp -r scripts/ "dist/${PACKAGE_DIR}/"

# Create installation script
cat > "dist/${PACKAGE_DIR}/install.sh" << 'EOF'
#!/bin/bash

echo "🚀 Installing Claude Usage"
echo "========================="

# Install Python version
echo "📦 Installing Python version..."
pip install -e .

# Install Rust version
echo "🦀 Installing Rust version..."
cd rust
cargo install --path .
cd ..

echo "✅ Installation complete!"
echo "   Python: python3 claude_usage.py"
echo "   Rust:   claude-usage"
EOF

chmod +x "dist/${PACKAGE_DIR}/install.sh"

# Create archive
cd dist
tar -czf "${PACKAGE_DIR}.tar.gz" "${PACKAGE_DIR}"
zip -r "${PACKAGE_DIR}.zip" "${PACKAGE_DIR}"

echo "✅ Package created:"
echo "   dist/${PACKAGE_DIR}.tar.gz"
echo "   dist/${PACKAGE_DIR}.zip"