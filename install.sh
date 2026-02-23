#!/bin/bash

# Paste!t Installer for Ubuntu 24.04 X11
# Optimizes for build time and setup speed.

set -e

echo "🔍 Checking dependencies..."
# Ignore errors if some 3rd party repos have GPG issues
sudo apt update || echo "⚠️ Warning: apt update had some errors, trying to proceed..."
sudo apt install -y cargo rustc libgtk-4-dev xdotool x11-utils sqlite3 libsqlite3-dev pkg-config || echo "⚠️ Warning: Some packages failed to install. If dependencies are already met, the build may still work."

# Priority 1: Check for repo-provided pre-built binary
if [ -f "./bin/paste-it" ]; then
    echo "✨ Pre-built binary found (14MB). Installing instantly..."
    SKIP_BUILD=true
    mkdir -p target/release/
    cp ./bin/paste-it ./target/release/
# Priority 2: Check for locally built binary
elif [ -f "./target/release/paste-it" ]; then
    read -p "🔄 Locally built binary found. Use it to skip build? (y/n) " use_prebuilt
    if [[ "$use_prebuilt" == "y" ]]; then
        SKIP_BUILD=true
    fi
fi

if [ "$SKIP_BUILD" != true ]; then
    echo "⚡ Building Paste!t from source (this may take 3-5 minutes)..."
    cargo build --release
fi

# Create local bin if it doesn't exist
mkdir -p ~/.local/bin
cp target/release/paste-it ~/.local/bin/

echo "📦 Setting up systemd service..."
mkdir -p ~/.config/systemd/user/

cat <<EOF > ~/.config/systemd/user/paste-it.service
[Unit]
Description=Paste!t Clipboard Manager Daemon
After=graphical-session.target

[Service]
ExecStart=%h/.local/bin/paste-it --daemon
Restart=always
Environment=DISPLAY=:0

[Install]
WantedBy=graphical-session.target
EOF

systemctl --user daemon-reload
systemctl --user enable paste-it.service
systemctl --user restart paste-it.service

echo "⌨️ Configuring Super+V shortcut..."
PATH_BASE="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/paste-it/"
existing_bindings=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings)

# Robustly append to gsettings array
if [[ "$existing_bindings" == "@as []" ]]; then
    new_bindings="['$PATH_BASE']"
elif [[ "$existing_bindings" != *"$PATH_BASE"* ]]; then
    new_bindings="${existing_bindings%]*}, '$PATH_BASE']"
else
    new_bindings="$existing_bindings"
fi

gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_BASE name "Paste!t Popup"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_BASE command "$HOME/.local/bin/paste-it"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$PATH_BASE binding "<Super>v"
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$new_bindings"

echo "✅ Installation complete!"
echo "Try copying something, then press Super+V to open the clipboard manager."
