use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let overlay_opacity = 0.6;
            
            let window = app.get_webview_window("main").unwrap();

            // The JavaScript code you want to inject
            let js_code = format!(r#"
                function updateOpacityOnFrame(timestamp) {{
                    const style = document.createElement('style');
                    style.innerHTML = `
                        html,
                        body {{
                            opacity: {overlay_opacity} !important;
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
