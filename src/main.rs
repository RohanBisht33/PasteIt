mod database;
mod daemon;
mod ui;
mod paste_handler;

use std::sync::Arc;
use std::env;
use std::io::{Write, Read};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::fs;
use database::Database;
use daemon::Daemon;
use ui::ClipboardUI;
use paste_handler::PasteHandler;

const SOCKET_PATH: &str = "/tmp/paste_it.sock";

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let db = Arc::new(Database::new()?);
    
    // Trigger cleanup on startup
    let _ = db.cleanup();
    
    let daemon = Arc::new(Daemon::new(db.clone()));

    gtk4::init().expect("Failed to initialize GTK");

    if args.len() > 1 && args[1] == "--daemon" {
        run_daemon(daemon);
    } else {
        run_ui(db, daemon);
    }

    Ok(())
}

fn run_daemon(daemon: Arc<Daemon>) {
    println!("Starting Paste!t Daemon...");
    
    // Minimal GTK setup for daemon to handle clipboard signals
    let display = gdk4::Display::default().expect("Could not connect to a display.");
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(include_str!("style.css"));
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    daemon.start();

    // Daemon Socket for persistent clipboard commands
    let daemon_socket = daemon.clone();
    let _ = fs::remove_file("/tmp/paste_it_daemon.sock");
    std::thread::spawn(move || {
        if let Ok(listener) = UnixListener::bind("/tmp/paste_it_daemon.sock") {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let mut buffer = String::new();
                    if let Ok(_) = stream.read_to_string(&mut buffer) {
                        if buffer.starts_with("SET:") {
                            let hash = buffer[4..].to_string();
                            daemon_socket.set_clipboard_from_db(hash);
                        }
                    }
                }
            }
        }
    });
    
    let main_context = gtk4::glib::MainContext::default();
    let _loop = gtk4::glib::MainLoop::new(Some(&main_context), false);
    _loop.run();
}

fn run_ui(db: Arc<Database>, daemon: Arc<Daemon>) {
    // 1. Singleton/Toggle Logic
    if Path::new(SOCKET_PATH).exists() {
        if let Ok(mut stream) = UnixStream::connect(SOCKET_PATH) {
            println!("UI already running. Sending toggle signal...");
            let _ = stream.write_all(b"QUIT");
            return; 
        } else {
            // Stale socket
            let _ = fs::remove_file(SOCKET_PATH);
        }
    }

    // 2. Bind Listener
    let listener = UnixListener::bind(SOCKET_PATH).expect("Failed to bind socket");
    listener.set_nonblocking(true).expect("Cannot set non-blocking");

    // 3. Setup UI
    let ui = ClipboardUI::new(db, daemon);
    let prev_window_id = PasteHandler::get_active_window_id().ok();

    // Setup CSS
    let display = gdk4::Display::default().expect("Could not connect to a display.");
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(include_str!("style.css"));
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // 4. Watch for Remote Toggle
    let (tx, rx) = gtk4::glib::MainContext::channel(gtk4::glib::Priority::DEFAULT);
    std::thread::spawn(move || {
        loop {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0; 4];
                if stream.read_exact(&mut buf).is_ok() && &buf == b"QUIT" {
                    let _ = tx.send(());
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });

    ui.run(prev_window_id, rx);

    // 5. Cleanup
    let _ = fs::remove_file(SOCKET_PATH);
}
