use crate::database::Database;
use crate::paste_handler::PasteHandler;
use crate::daemon::Daemon;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Orientation, Label, Image, ScrolledWindow, EventControllerKey, PolicyType, ListBox, GestureDrag, Button};
use std::sync::Arc;
use gdk4::Display;
use std::collections::HashMap;
use std::sync::Mutex;
use gtk4::glib::clone;

pub struct ClipboardUI {
    db: Arc<Database>,
    daemon: Arc<Daemon>,
}

impl ClipboardUI {
    pub fn new(db: Arc<Database>, daemon: Arc<Daemon>) -> Self {
        ClipboardUI { db, daemon }
    }

    pub fn run(&self, prev_window_id: Option<String>, toggle_rx: async_channel::Receiver<()>) {
        let app = gtk4::Application::builder()
            .application_id("com.paste.it.ui")
            .build();

        let db = self.db.clone();
        let _daemon = self.daemon.clone(); // kept for future use / struct symmetry
        let row_map = Arc::new(Mutex::new(HashMap::new()));
        let prev_id = prev_window_id.clone();

        let toggle_rx = Arc::new(Mutex::new(Some(toggle_rx))); // async_channel::Receiver<()>

        app.connect_activate(move |app| {
            let toggle_rx_inner = toggle_rx.lock().unwrap().take();
            let window = ApplicationWindow::builder()
                .application(app)
                .title("Clipboard history")
                .default_width(400)
                .default_height(600)
                .decorated(false)
                .resizable(false)
                .focusable(true)
                .build();

            window.set_css_classes(&["popup-window", "dark"]);
            
            // Flicker-free Positioning: Start invisible
            window.set_opacity(0.0);
            window.present();

            let title = "Clipboard history";
            let (show_tx, show_rx) = async_channel::bounded::<()>(1);
            let window_reveal = window.clone();
            gtk4::glib::MainContext::default().spawn_local(clone!(@strong show_rx => async move {
                if show_rx.recv().await.is_ok() {
                    window_reveal.set_opacity(1.0);
                }
            }));

            std::thread::spawn(move || {
                // Wait just enough for the window manager to realize the window
                std::thread::sleep(std::time::Duration::from_millis(60));
                
                let display = Display::default().expect("Could not get default display");
                let monitors = display.monitors();
                if let Some(monitor) = monitors.item(0).and_then(|m| m.downcast::<gdk4::Monitor>().ok()) {
                    let geometry = monitor.geometry();
                    let x = geometry.width() - 400 - 20; 
                    let y = geometry.height() - 600 - 60; 

                    // 1. Position the window while it's still invisible
                    let _ = std::process::Command::new("xdotool")
                        .arg("search")
                        .arg("--name")
                        .arg(title)
                        .arg("windowmove")
                        .arg(format!("{}", x))
                        .arg(format!("{}", y))
                        .status();
                    
                    // 2. Set X11 floating hints
                    let _ = std::process::Command::new("xprop")
                        .arg("-name")
                        .arg(title)
                        .arg("-f")
                        .arg("_NET_WM_STATE")
                        .arg("32a")
                        .arg("-set")
                        .arg("_NET_WM_STATE")
                        .arg("_NET_WM_STATE_SKIP_TASKBAR,_NET_WM_STATE_SKIP_PAGER,_NET_WM_STATE_ABOVE")
                        .status();

                    // 3. Signal main thread to reveal the window
                    let _ = show_tx.send_blocking(());
                }
            });

            let vbox = Box::new(Orientation::Vertical, 0);

            // 1. Drag handle bar (no emojis — just a clean draggable strip)
            let header_bar = Box::new(Orientation::Horizontal, 0);
            header_bar.set_css_classes(&["header-bar"]);
            vbox.append(&header_bar);

            // 2. Drag Gesture for Header
            let drag = GestureDrag::new();
            let last_offset = Arc::new(Mutex::new((0.0, 0.0)));
            
            let last_offset_begin = last_offset.clone();
            drag.connect_drag_begin(move |_, _, _| {
                *last_offset_begin.lock().unwrap() = (0.0, 0.0);
            });

            let last_offset_update = last_offset.clone();
            drag.connect_drag_update(move |_, offset_x, offset_y| {
                let mut last = last_offset_update.lock().unwrap();
                let dx = offset_x - last.0;
                let dy = offset_y - last.1;
                *last = (offset_x, offset_y);

                let title = "Clipboard history";
                let _ = std::process::Command::new("xdotool")
                    .arg("search")
                    .arg("--name")
                    .arg(title)
                    .arg("windowmove")
                    .arg("--relative")
                    .arg(format!("{}", dx as i32))
                    .arg(format!("{}", dy as i32))
                    .status();
            });
            header_bar.add_controller(drag);

            // 3. Sub-header (Label + Clear All)
            let sub_header = Box::new(Orientation::Horizontal, 0);
            sub_header.set_margin_start(16);
            sub_header.set_margin_end(16);
            sub_header.set_margin_top(10);
            sub_header.set_margin_bottom(10);

            let title_lbl = Label::new(Some("Clipboard"));
            title_lbl.set_css_classes(&["header-label"]);
            title_lbl.set_halign(gtk4::Align::Start);
            title_lbl.set_hexpand(true);
            sub_header.append(&title_lbl);

            let clear_btn = Button::with_label("Clear all");
            clear_btn.set_css_classes(&["clear-button"]);
            sub_header.append(&clear_btn);
            vbox.append(&sub_header);

            let scrolled_window = ScrolledWindow::builder()
                .hscrollbar_policy(PolicyType::Never)
                .vscrollbar_policy(PolicyType::Automatic)
                .vexpand(true)
                .build();
            
            let list_box = ListBox::new();
            list_box.set_selection_mode(gtk4::SelectionMode::Single);
            list_box.set_activate_on_single_click(true);
            scrolled_window.set_child(Some(&list_box));
            vbox.append(&scrolled_window);

            let db_clear_btn = db.clone();
            let list_box_clear = list_box.clone();
            let row_map_clear = row_map.clone();
            clear_btn.connect_clicked(move |_| {
                if let Ok(_) = db_clear_btn.clear_history() {
                    Self::refresh_list(&list_box_clear, db_clear_btn.clone(), &row_map_clear);
                }
            });

            window.set_child(Some(&vbox));

            // Populate list
            Self::refresh_list(&list_box, db.clone(), &row_map);

            // Close on focus loss
            let window_focus = window.clone();
            let focus_controller = gtk4::EventControllerFocus::new();
            focus_controller.connect_leave(move |_| {
                window_focus.close();
            });
            window.add_controller(focus_controller);

            // Force focus on launch
            window.present();
            window.set_focusable(true);
            window.grab_focus();

            // Keyboard navigation
            let window_key = window.clone();
            let key_controller = EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, keyval, _, _| {
                if keyval == gdk4::Key::Escape {
                    window_key.close();
                    gtk4::glib::Propagation::Stop
                } else {
                    gtk4::glib::Propagation::Proceed
                }
            });
            window.add_controller(key_controller);

            // Item activation
            let window_action = window.clone();
            let prev_id_action = prev_id.clone();
            let row_map_action = row_map.clone();

            list_box.connect_row_activated(move |_, row| {
                let index = row.index();
                let entry_data = {
                    let map = row_map_action.lock().unwrap();
                    map.get(&index).cloned()
                };

                if let Some(entry) = entry_data {
                    // Close the UI window first so focus can return to the target
                    window_action.close();

                    // Send the hash + target window ID to the daemon.
                    // The daemon will: set clipboard → wait 80ms → activate target
                    // window → inject Ctrl+V. This ensures we paste the correct
                    // entry (by hash) rather than whatever is most-recently set.
                    let wid = prev_id_action.as_deref();
                    if let Err(e) = PasteHandler::send_set_command(&entry.content_hash, wid) {
                        eprintln!("send_set_command error: {}", e);
                    }
                }
            });

            // Handle Toggle Signal
            if let Some(rx) = toggle_rx_inner {
                let window_toggle = window.clone();
                gtk4::glib::MainContext::default().spawn_local(async move {
                    if rx.recv().await.is_ok() {
                        window_toggle.close();
                    }
                });
            }

            window.present();
        });

        app.run_with_args::<&str>(&[]);
    }

    fn refresh_list(list_box: &ListBox, db: Arc<Database>, row_map: &Arc<Mutex<HashMap<i32, crate::database::ClipboardEntry>>>) {
        while let Some(row) = list_box.last_child() {
            list_box.remove(&row);
        }

        let mut map = row_map.lock().unwrap();
        map.clear();

        if let Ok(history) = db.get_history(None) {
            if history.is_empty() {
                let empty_lbl = Label::new(Some("Your clipboard history is empty.\nCopy something to see it here!"));
                empty_lbl.set_css_classes(&["clip-text"]);
                empty_lbl.set_margin_top(40);
                empty_lbl.set_opacity(0.6);
                list_box.append(&empty_lbl);
                return;
            }
            for (i, entry) in history.into_iter().enumerate() {
                let main_card_box = Box::new(Orientation::Horizontal, 0);
                main_card_box.set_css_classes(&["clip-card"]);

                let content_box = Box::new(Orientation::Vertical, 4);
                content_box.set_hexpand(true);
                
                if entry.entry_type == "image" {
                    if let Some(thumb_bytes) = &entry.thumbnail {
                        let loader = gdk_pixbuf::PixbufLoader::new();
                        let _ = loader.write(thumb_bytes);
                        let _ = loader.close();
                        if let Some(pixbuf) = loader.pixbuf() {
                            let texture = gdk4::Texture::for_pixbuf(&pixbuf);
                            let img_widget = Image::from_paintable(Some(&texture));
                            img_widget.set_css_classes(&["clip-image"]);
                            img_widget.set_pixel_size(100);
                            content_box.append(&img_widget);
                        }
                    }
                } else {
                    let text = String::from_utf8_lossy(&entry.content);
                    let label = Label::new(Some(&text.lines().take(2).collect::<Vec<_>>().join("\n")));
                    label.set_css_classes(&["clip-text"]);
                    label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                    label.set_halign(gtk4::Align::Start);
                    label.set_max_width_chars(40);
                    content_box.append(&label);
                }

                // Controls Box (Right side)
                let controls_box = Box::new(Orientation::Vertical, 0);
                controls_box.set_css_classes(&["card-controls"]);
                
                let menu_btn = Button::builder().label("✕").has_frame(false).build();
                menu_btn.set_css_classes(&["action-btn", "delete-btn"]);
                
                let pin_label = if entry.pinned { "★" } else { "☆" };
                let pin_btn = Button::builder().label(pin_label).has_frame(false).build();
                pin_btn.set_css_classes(&["action-btn", "pin-btn"]);
                if entry.pinned {
                    pin_btn.add_css_class("pinned");
                }

                // Callbacks
                let db_pin = db.clone();
                let list_box_pin = list_box.clone();
                let row_map_pin = row_map.clone();
                let entry_id = entry.id;
                pin_btn.connect_clicked(move |_| {
                    if let Ok(_) = db_pin.toggle_pin(entry_id) {
                        Self::refresh_list(&list_box_pin, db_pin.clone(), &row_map_pin);
                    }
                });

                let db_del = db.clone();
                let list_box_del = list_box.clone();
                let row_map_del = row_map.clone();
                menu_btn.connect_clicked(move |btn| {
                    // Walk up widget tree to find the root ApplicationWindow
                    let parent_window = btn.root()
                        .and_then(|r| r.downcast::<gtk4::Window>().ok());

                    let dialog = gtk4::MessageDialog::new(
                        parent_window.as_ref(),
                        gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
                        gtk4::MessageType::Warning,
                        gtk4::ButtonsType::None,
                        "Delete this clip?",
                    );
                    dialog.set_secondary_text(Some("This action cannot be undone."));
                    dialog.add_button("Cancel", gtk4::ResponseType::Cancel);
                    let del_btn = dialog.add_button("Delete", gtk4::ResponseType::Accept);
                    del_btn.add_css_class("destructive-action");

                    let db_del2 = db_del.clone();
                    let list_box_del2 = list_box_del.clone();
                    let row_map_del2 = row_map_del.clone();
                    dialog.connect_response(move |dlg, resp| {
                        if resp == gtk4::ResponseType::Accept {
                            if db_del2.delete_entry(entry_id).is_ok() {
                                Self::refresh_list(&list_box_del2, db_del2.clone(), &row_map_del2);
                            }
                        }
                        dlg.close();
                    });
                    dialog.show();
                });

                controls_box.append(&menu_btn);
                let spacer = Box::new(Orientation::Vertical, 0);
                spacer.set_vexpand(true);
                controls_box.append(&spacer);
                controls_box.append(&pin_btn);

                main_card_box.append(&content_box);
                main_card_box.append(&controls_box);

                let row = gtk4::ListBoxRow::new();
                row.set_child(Some(&main_card_box));
                list_box.append(&row);
                
                map.insert(i as i32, entry);
            }
        }
    }
}