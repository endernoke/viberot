use std::io::{BufRead, BufReader};
use std::thread;
use tauri::Manager;
use tauri_plugin_cli::CliExt;
use url::Url;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_cli::init())
        .setup(|app| {
            let mut overlay_opacity = 0.6;
            let mut overlay_url = Url::parse("https://www.tiktok.com/foryou").unwrap();
            match app.cli().matches() {
                // `matches` here is a Struct with { args, subcommand }.
                // `args` is `HashMap<String, ArgData>` where `ArgData` is a struct with { value, occurrences }.
                // `subcommand` is `Option<Box<SubcommandMatches>>` where `SubcommandMatches` is a struct with { name, matches }.
                Ok(matches) => {
                    if let Some(opacity_arg) = matches.args.get("opacity") {
                        if let Some(opacity_str) = opacity_arg.value.as_str() {
                            if let Ok(opacity) = opacity_str.parse::<f64>() {
                                if opacity >= 0.0 && opacity <= 1.0 {
                                    overlay_opacity = opacity;
                                }
                            }
                        }
                    }
                    
                    if let Some(url_arg) = matches.args.get("url") {
                        if let Some(url_str) = url_arg.value.as_str() {
                            overlay_url = Url::parse(url_str).unwrap();
                        }
                    }
                }
                Err(_) => {}
            }

            // Get app handle for terminating the application
            let app_handle = app.handle().clone();
            setup_stdin_monitor(app_handle);
            let window = app.get_webview_window("main").unwrap();
            window.navigate(overlay_url).unwrap();
            // Window transparency only affects the window frame, not the web content.
            // so we need to inject some JS/CSS to make the web content transparent as well.
            configure_window_content_opacity(&window, overlay_opacity);

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
                div {{
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
