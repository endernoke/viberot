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
                            overlay_url = Url::parse(url_str).unwrap_or(overlay_url.clone());
                        }
                    }

                    if let Some(stdin_arg) = matches.args.get("exit-on-stdin-close") {
                        if let Some(stdin_value) = stdin_arg.value.as_bool() {
                            if stdin_value {
                                setup_stdin_monitor(app.handle().clone());
                            }
                        }
                    }
                }
                Err(_) => {}
            }

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let _ = tauri::WebviewWindowBuilder::new(&handle, "main", tauri::WebviewUrl::External(overlay_url))
                    .initialization_script(format!(r#"
                        function updateOpacityOnFrame(timestamp) {{
                            const style = document.createElement('style');
                            style.innerHTML = `
                                html,
                                body {{
                                    opacity: {overlay_opacity} !important;
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
                    "#))
                    .title("VibeRot Overlay")
                    .transparent(true)
                    .fullscreen(true)
                    .resizable(false)
                    .always_on_top(true)
                    .decorations(false)
                    .build()
                    .unwrap();
            });
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
                    // Close window first to avoid resource leaks
                    app_handle.get_webview_window("main").unwrap().close().unwrap();
                    app_handle.exit(1);
                    return;
                }
            }
        }

        // If we exit the loop naturally (stdin closed)
        eprintln!("stdin closed, terminating application");
        // Close window first to avoid resource leaks
        app_handle.get_webview_window("main").unwrap().close().unwrap();
        app_handle.exit(0);
    });
}
