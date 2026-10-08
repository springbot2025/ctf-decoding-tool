#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::decode_cmd,
            commands::find_flag_cmd,
            commands::caesar_brute_cmd,
            commands::auto_detect_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 启动失败");
}
