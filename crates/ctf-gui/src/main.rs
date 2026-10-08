#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

// FIXME: 本 crate 与 frontend/src-tauri 完全重复（逐字节相同），
// 改命令需两边同步，建议删除本 crate 只保留 frontend/src-tauri
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
