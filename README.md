# 🚀 Paste!t

### High-Fidelity Windows 11 Clipboard Replica for Ubuntu (X11)

**Paste!t** is a powerful, lightweight, and visually stunning clipboard manager designed to replicate the Windows 11 **Win+V** experience on Ubuntu 24.04 X11.

![Theme](https://img.shields.io/badge/Theme-Windows%2011%20Light-blue)
![Language](https://img.shields.io/badge/Language-Rust-orange)
![Toolkit](https://img.shields.io/badge/Toolkit-GTK4-green)

---

## ✨ Features

- **🏆 High-Fidelity UI**: Solid Windows 11 Light Theme with icon header navigation.
- **� Smart Pinning**: Pin important items with a dedicated blue-active state icon.
- **⋮ Item Management**: Individual item deletion via the three-dots menu.
- **🧹 Clear All**: Instantly wipe unpinned history with one click.
- **🖼 Image Thumbnails**: Optimized 128px previews for copied images.
- **⚡ Intentional Paste**: Selection manager that restores focus and auto-pastes seamlessly.
- **🎯 Draggable**: Move the clipboard window anywhere via the top icon bar.
- **🔒 Deduplication**: SHA256 hashing prevents duplicate clutter.

---

## 🛠 Prerequisites

Ensure you have the following tools installed on your Ubuntu system:

```bash
sudo apt update
sudo apt install -y cargo rustc libgtk-4-dev xdotool xprop sqlite3 libsqlite3-dev pkg-config
```

---

## ⚙️ Installation

1. **Clone and Build**:
   ```bash
   git clone https://github.com/RohanBisht33/PasteIt.git
   cd PasteIt
   cargo build --release
   ```

2. **Install to Local Bin**:
   ```bash
   mkdir -p ~/.local/bin
   cp target/release/paste-it ~/.local/bin/
   ```

3. **Install Service**:
   Setup the daemon to run automatically on login:
   ```bash
   mkdir -p ~/.config/systemd/user/
   cat <<EOF > ~/.config/systemd/user/paste-it.service
   [Unit]
   Description=Paste!t Clipboard Manager Daemon
   After=graphical-session.target

   [Service]
   ExecStart=%h/.local/bin/paste-it --daemon
   Restart=always

   [Install]
   WantedBy=main.target
   EOF

   systemctl --user daemon-reload
   systemctl --user enable paste-it.service
   systemctl --user start paste-it.service
   ```

---

## 🎹 Global Shortcut (Super+V)

To trigger the clipboard manager like Windows:

1. Open **Settings** -> **Keyboard Shortcuts**.
2. Add a **Custom Shortcut**:
   - **Name**: `Paste!t UI`
   - **Command**: `~/.local/bin/paste-it`
   - **Shortcut**: `Super+V`

---

## 🗑️ Uninstallation

If you wish to remove Paste!t completely:

```bash
# 1. Stop and disable the service
systemctl --user stop paste-it.service
systemctl --user disable paste-it.service

# 2. Remove binary and service file
rm ~/.local/bin/paste-it
rm ~/.config/systemd/user/paste-it.service

# 3. Reload systemd
systemctl --user daemon-reload

# 4. (Optional) Remove database
rm ~/clipboard_history.db
```

---

## 📝 License
MIT License. Feel free to contribute!
