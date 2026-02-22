#!/bin/bash

# Paste!t Installer for Ubuntu 24.04 X11

set -e

echo "🚀 Building Paste!t..."
cargo build --release

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
systemctl --user start paste-it.service

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
