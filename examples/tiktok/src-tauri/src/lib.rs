use tauri::Manager;
use std::thread;
use std::io::{BufRead, BufReader};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let overlay_opacity = 0.6;
            // Window transparency only affects the window frame, not the web content.
            // so we need to inject some JS/CSS to make the web content transparent as well.
            configure_window_content_opacity(&window, overlay_opacity);

            // Get app handle for terminating the application
            let app_handle = app.handle().clone();
            setup_stdin_monitor(app_handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_stdin_monitor(app_handle: tauri::AppHandle) {
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let mut line = String::new();

        loop {
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF reached
                    break;
                }
                Ok(_) => {
                    // Successfully read a line, continue
                }
                Err(e) => {
                    // Error reading from stdin (likely closed)
                    eprintln!("Error reading stdin: {}, terminating application", e);
                    app_handle.exit(1);
                    return;
                }
            }
        }
        
        // If we exit the loop naturally (stdin closed)
        eprintln!("stdin closed, terminating application");
        app_handle.exit(0);
    });
}

fn configure_window_content_opacity(window: &tauri::WebviewWindow, opacity: f64) {
    let js_code = format!(r#"
        function updateOpacityOnFrame(timestamp) {{
            const style = document.createElement('style');
            style.innerHTML = `
                html,
                body {{
                    opacity: {opacity} !important;
                    background-color: transparent !important;
                }}
            `;
            document.head.appendChild(style);

            // Request the next animation frame to create a loop
            requestAnimationFrame(updateStyleOnFrame);
        }};
        // Start the loop
        requestAnimationFrame(updateOpacityOnFrame);
    "#);
    
    // Evaluate the JS code in the webview
    window.eval(js_code).unwrap();
}
