# 🚀 Paste!t

### Windows Win+V Clipboard History for Ubuntu (X11)

<p>
<strong>Rust • GTK4 • SQLite • X11</strong>
</p>

---

## 📌 Overview

Paste!t replicates the Windows 11 **Win + V** clipboard history experience on Ubuntu 24.04 (X11).

Persistent clipboard storage.  
Pinned items.  
Image thumbnails.  
Instant click-to-paste.  
Built in Rust for performance and memory safety.

---

## ✨ Features

- 📋 Clipboard history (Text, HTML, URLs, Images)
- 📌 Pin important entries
- 🖼 128px optimized image thumbnails
- ⚡ Click to paste (no manual Ctrl+V required)
- 🔒 SHA256-based deduplication
- 🧠 Smart cleanup (500 item limit, pinned protected)
- 🎯 Super+V global shortcut
- 🧵 Zero memory leaks (Rust)

---

## 🖥 UI Behavior

- Bottom-right floating popup (Windows-style)
- Rounded corners (12px)
- 2-line text preview
- Image thumbnail previews
- Closes automatically after paste
- Hidden from taskbar and Alt+Tab

---

## ⚙️ Installation

### 1️⃣ Install Dependencies

```bash
sudo apt update
sudo apt install -y cargo rustc libgtk-4-dev xclip xdotool sqlite3 libsqlite3-dev pkg-config
cargo build --release
./target/release/paste_it
```
