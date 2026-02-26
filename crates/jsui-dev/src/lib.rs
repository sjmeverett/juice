use std::sync::mpsc;
use std::time::Duration;

/// Check for a `DEV_SERVER` environment variable and, if set, spawn a background
/// thread that connects to the WebSocket dev server and receives new bundles.
///
/// Returns an `mpsc::Receiver<String>` — call `try_recv()` each frame in your
/// event loop. When a new bundle arrives, drop the old Engine, recreate it, and
/// boot with the new bundle.
///
/// If `DEV_SERVER` is not set, returns a receiver that never produces a message.
pub fn spawn_reload_listener() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();

    if let Ok(dev_url) = std::env::var("DEV_SERVER") {
        std::thread::spawn(move || {
            loop {
                match tungstenite::connect(&dev_url) {
                    Ok((mut socket, _)) => {
                        println!("[dev] connected to {}", dev_url);
                        loop {
                            match socket.read() {
                                Ok(tungstenite::Message::Text(bundle)) => {
                                    if tx.send(bundle.into()).is_err() {
                                        return;
                                    }
                                }
                                Ok(tungstenite::Message::Close(_)) | Err(_) => break,
                                _ => {}
                            }
                        }
                        println!("[dev] disconnected, reconnecting...");
                    }
                    Err(e) => {
                        eprintln!("[dev] connect failed: {e}, retrying in 1s");
                    }
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }

    rx
}
