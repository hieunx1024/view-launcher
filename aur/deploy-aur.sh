#!/bin/bash
set -e

AUR_REPO="ssh://aur@aur.archlinux.org/view-launcher-git.git"
CLONE_DIR="/tmp/view-launcher-git-aur"

echo "=== AUR Deployment Helper ==="
echo "Make sure you have registered your SSH key at https://aur.archlinux.org"
echo ""

# Clean previous clone if any
rm -rf "$CLONE_DIR"

echo "Cloning AUR repository..."
git clone "$AUR_REPO" "$CLONE_DIR"

echo "Copying PKGBUILD and .SRCINFO..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$SCRIPT_DIR/PKGBUILD" "$SCRIPT_DIR/.SRCINFO" "$CLONE_DIR/"

cd "$CLONE_DIR"

echo "Current git status in AUR repo:"
git status

echo ""
echo "To publish, run the following commands:"
echo "  cd $CLONE_DIR"
echo "  git add PKGBUILD .SRCINFO"
echo "  git commit -m \"Update to latest master commit\""
echo "  git push origin master"
echo ""
