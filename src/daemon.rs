use crate::database::Database;
use gtk4::prelude::*;
use gdk4::Texture;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::io::Cursor;
use image::imageops::FilterType;

pub struct Daemon {
    db: Arc<Database>,
    last_injected_hash: Arc<Mutex<Option<String>>>,
}

impl Daemon {
    pub fn new(db: Arc<Database>) -> Self {
        Daemon {
            db,
            last_injected_hash: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&self) {
        let display = gdk4::Display::default().expect("Could not connect to a display.");
        let clipboard = display.clipboard();

        let db = self.db.clone();
        let last_injected = self.last_injected_hash.clone();

        clipboard.connect_changed(move |cb| {
            let db_text = db.clone();
            let last_injected_text = last_injected.clone();

            // Handle text
            cb.read_value_async(gtk4::glib::Type::STRING, gtk4::glib::Priority::DEFAULT, None::<&gtk4::gio::Cancellable>, move |res| {
                if let Ok(value) = res {
                    if let Ok(text) = value.get::<String>() {
                        Self::handle_text(text, db_text.clone(), last_injected_text.clone());
                    }
                }
            });

            let db_img = db.clone();
            let last_injected_img = last_injected.clone();

            // Handle images
            cb.read_texture_async(None::<&gtk4::gio::Cancellable>, move |res| {
                if let Ok(Some(texture)) = res {
                    Self::handle_image(texture, db_img.clone(), last_injected_img.clone());
                }
            });
        });
    }

    fn handle_text(text: String, db: Arc<Database>, last_injected: Arc<Mutex<Option<String>>>) {
        let content = text.as_bytes();
        let hash = Self::calculate_hash(content, "text");

        {
            let mut lh = last_injected.lock().unwrap();
            if Some(hash.clone()) == *lh {
                *lh = None; // Reset after one skip
                return; // Loop protection
            }
        }

        let db_thread = db.clone();
        let content_vec = content.to_vec();
        std::thread::spawn(move || {
            if let Err(e) = db_thread.add_entry(&content_vec, None, "text") {
                eprintln!("Error saving text clipboard: {}", e);
            }
        });
    }

    fn handle_image(texture: Texture, db: Arc<Database>, last_injected: Arc<Mutex<Option<String>>>) {
        let db_thread = db.clone();
        let last_injected_clone = last_injected.clone();

        let pixbuf = gdk4::pixbuf_get_from_texture(&texture).expect("Failed to get pixbuf from texture");
        
        if let Ok(buffer) = pixbuf.save_to_bufferv("png", &[]) {
            let content = buffer;
            let hash = Self::calculate_hash(&content, "image");

            {
                let mut lh = last_injected_clone.lock().unwrap();
                if Some(hash.clone()) == *lh {
                    *lh = None;
                    return;
                }
            }

            // Async processing for image
            std::thread::spawn(move || {
                // Generate thumbnail in background worker
                let thumbnail = match image::load_from_memory(&content) {
                    Ok(img) => {
                        // Use Lanczos3 as requested
                        let thumb = img.resize(128, 128, FilterType::Lanczos3);
                        let mut thumb_bytes = Vec::new();
                        let mut cursor = Cursor::new(&mut thumb_bytes);
                        if thumb.write_to(&mut cursor, image::ImageFormat::Png).is_ok() {
                            Some(thumb_bytes)
                        } else {
                            None
                        }
                    },
                    Err(_) => None,
                };

                if let Err(e) = db_thread.add_entry(&content, thumbnail.as_deref(), "image") {
                    eprintln!("Error saving image clipboard: {}", e);
                }
            });
        }
    }

    fn calculate_hash(content: &[u8], entry_type: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hasher.update(entry_type.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn set_clipboard_from_db(&self, hash: String) {
        if let Ok(history) = self.db.get_history(None) {
            if let Some(entry) = history.into_iter().find(|e| e.content_hash == hash) {
                if entry.entry_type == "text" {
                    let text = String::from_utf8_lossy(&entry.content).to_string();
                    let display = gdk4::Display::default().expect("No display");
                    let clipboard = display.clipboard();
                    
                    self.set_last_injected_hash(hash);
                    clipboard.set_text(&text);
                }
            }
        }
    }

    pub fn set_last_injected_hash(&self, hash: String) {
        let mut lh = self.last_injected_hash.lock().unwrap();
        *lh = Some(hash);
    }
}
