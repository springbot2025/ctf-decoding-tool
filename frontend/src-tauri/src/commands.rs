use ctf_core::{decode, caesar_brute_force, auto_detect, find_flag, DetectResult};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct DecodeRequest {
    pub method: String,
    pub cipher: String,
    /// 可选键值对参数，如 key、shift、iv、a、b、rails 等
    #[serde(default)]
    pub params: HashMap<String, String>,
}

/// 通用解码/解密命令，结果字符串或错误信息
#[tauri::command]
pub fn decode_cmd(req: DecodeRequest) -> Result<String, String> {
    decode(&req.method, &req.cipher, &req.params).map_err(|e| e.to_string())
}

/// 在长文本中寻找 flag
#[tauri::command]
pub fn find_flag_cmd(text: String, header: String) -> Option<String> {
    find_flag(&text, &header)
}

/// Caesar 暴力枚举：返回 26 条 (shift, plaintext) 元组
#[tauri::command]
pub fn caesar_brute_cmd(cipher: String) -> Vec<(u8, String)> {
    caesar_brute_force(&cipher)
}

/// 自动识别：并发尝试所有无密钥方法，返回评分排序结果
#[tauri::command]
pub fn auto_detect_cmd(cipher: String) -> Vec<DetectResult> {
    auto_detect(&cipher)
}
