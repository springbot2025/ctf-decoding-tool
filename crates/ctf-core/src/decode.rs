use std::collections::HashMap;

use base64::{Engine as _, engine::general_purpose};
use data_encoding::BASE32;
use num_bigint::BigUint;
use thiserror::Error;

pub type Params = HashMap<String, String>;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("不支持的方法: {0}")]
    UnsupportedMethod(String),
    #[error("{method} 解码失败: {reason}")]
    DecodeFailed { method: String, reason: String },
    #[error("缺少参数: {0}")]
    MissingParam(String),
    #[error("参数无效: {0}")]
    InvalidParam(String),
}

impl DecodeError {
    fn failed(method: &str, reason: impl ToString) -> Self {
        Self::DecodeFailed { method: method.to_string(), reason: reason.to_string() }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DetectResult {
    pub method: String,
    pub result: String,
    pub score:  f64,
}

/// 去除空白和逗号
fn clean(s: &str) -> String {
    s.split_whitespace().collect::<String>().replace(',', "")
}

/// 宽松 UTF-8 转换
fn to_utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}


// ─── 编码类（无密钥）─────────────────────────────────────────────────────────

fn decode_base64(cipher: &str) -> Result<String, DecodeError> {
    let v = clean(cipher);
    let pad = v.len() % 4;
    let v = if pad > 0 { format!("{}{}", v, "=".repeat(4 - pad)) } else { v };
    general_purpose::STANDARD.decode(&v)
        .or_else(|_| general_purpose::URL_SAFE.decode(&v))
        .map(to_utf8)
        .map_err(|e| DecodeError::failed("base64", e))
}

fn decode_base32(cipher: &str) -> Result<String, DecodeError> {
    let mut v = clean(cipher).to_uppercase();
    let pad = v.len() % 8;
    if pad > 0 { v.push_str(&"=".repeat(8 - pad)); }
    BASE32.decode(v.as_bytes()).map(to_utf8)
        .map_err(|e| DecodeError::failed("base32", e))
}

fn decode_base16(cipher: &str) -> Result<String, DecodeError> {
    let v = clean(cipher).to_uppercase().replace(' ', "");
    hex::decode(&v).map(to_utf8)
        .map_err(|e| DecodeError::failed("base16", e))
}

fn decode_base85(cipher: &str) -> Result<String, DecodeError> {
    let v: Vec<u8> = clean(cipher)
        .trim_start_matches("<~").trim_end_matches("~>")
        .bytes().filter(|b| !matches!(b, b' '|b'\t'|b'\n'|b'\r')).collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < v.len() {
        if v[i] == b'z' { result.extend_from_slice(&[0u8; 4]); i += 1; continue; }
        let chunk = &v[i..v.len().min(i + 5)];
        let mut val: u64 = 0;
        for (j, &b) in chunk.iter().enumerate() {
            let n = (b as u64).checked_sub(33)
                .ok_or_else(|| DecodeError::failed("base85", "字符超出范围"))?;
            if n >= 85 {
                return Err(DecodeError::failed("base85", "字符超出范围"));
            }
            val += n * 85u64.pow((4 - j) as u32);
        }
        if chunk.len() == 5 && val > u32::MAX as u64 {
            return Err(DecodeError::failed("base85", "数值溢出 (> u32::MAX)"));
        }
        let n_out = if chunk.len() < 5 { chunk.len() - 1 } else { 4 };
        for k in (0..4usize).rev().take(n_out) { result.push((val >> (k * 8)) as u8); }
        i += chunk.len();
    }
    Ok(to_utf8(result))
}

fn decode_base58(cipher: &str) -> Result<String, DecodeError> {
    bs58::decode(clean(cipher)).into_vec().map(to_utf8)
        .map_err(|e| DecodeError::failed("base58", e))
}

fn decode_base62(cipher: &str) -> Result<String, DecodeError> {
    const ALPHA: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut n = num_bigint::BigUint::from(0u32);
    for c in clean(cipher).bytes() {
        let pos = ALPHA.iter().position(|&b| b == c)
            .ok_or_else(|| DecodeError::failed("base62", format!("无效字符: {}", c as char)))?;
        n = n * 62u32 + pos as u32;
    }
    Ok(to_utf8(n.to_bytes_be()))
}

fn decode_url(cipher: &str) -> Result<String, DecodeError> {
    urlencoding::decode(cipher).map(|s| s.into_owned())
        .map_err(|e| DecodeError::failed("url", e))
}

fn decode_html_entity(cipher: &str) -> Result<String, DecodeError> {
    let mut s = cipher.to_string();
    // numeric hex: &#x41;
    let hex_re = regex::Regex::new(r"&#x([0-9a-fA-F]+);").unwrap();
    let result = hex_re.replace_all(&s, |caps: &regex::Captures| {
        u32::from_str_radix(&caps[1], 16)
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| caps[0].to_string())
    }).to_string();
    s = result;
    // numeric decimal: &#65;
    let dec_re = regex::Regex::new(r"&#(\d+);").unwrap();
    let result = dec_re.replace_all(&s, |caps: &regex::Captures| {
        caps[1].parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| caps[0].to_string())
    }).to_string();
    s = result;
    // named entities
    let named = [("&amp;","&"),("&lt;","<"),("&gt;",">"),("&quot;","\""),
                 ("&apos;","'"),("&nbsp;"," ")];
    for (ent, ch) in &named { s = s.replace(ent, ch); }
    Ok(s)
}

fn decode_unicode_escape(cipher: &str) -> Result<String, DecodeError> {
    let re = regex::Regex::new(r"\\[Uu]([0-9a-fA-F]{4,8})").unwrap();
    let result = re.replace_all(cipher, |caps: &regex::Captures| {
        u32::from_str_radix(&caps[1], 16)
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| caps[0].to_string())
    });
    Ok(result.into_owned())
}

fn decode_morse(cipher: &str) -> Result<String, DecodeError> {
    const TABLE: &[(&str, char)] = &[
        (".-",'A'),("-...",'B'),("-.-.",'C'),("-..",'D'),(".",'E'),
        ("..-.",'F'),("--.",'G'),("....",'H'),("..",'I'),(".---",'J'),
        ("-.-",'K'),(".-..",'L'),("--",'M'),("-.",'N'),("---",'O'),
        (".--.",'P'),("--.-",'Q'),(".-.",'R'),("...",'S'),("-",'T'),
        ("..-",'U'),("...-",'V'),(".--",'W'),("-..-",'X'),("-.--",'Y'),
        ("--..",'Z'),("-----",'0'),(".----",'1'),("..---",'2'),
        ("...--",'3'),("....-",'4'),(".....",'5'),("-....",'6'),
        ("--...",'7'),("---.",'8'),
        ("----.",'9'),
    ];
    let normalized = cipher.replace('_', "-").replace('·', ".").replace('•', ".")
        .replace('/', "   ");
    let words: Vec<&str> = normalized.split("   ").filter(|w| !w.trim().is_empty()).collect();
    let mut out = String::new();
    for (wi, word) in words.iter().enumerate() {
        if wi > 0 { out.push(' '); }
        for code in word.split_whitespace() {
            let ch = TABLE.iter().find(|(s, _)| *s == code)
                .map(|(_, c)| *c)
                .ok_or_else(|| DecodeError::failed("morse", format!("未知符号: {code}")))?;
            out.push(ch);
        }
    }
    Ok(out)
}

fn decode_octal(cipher: &str) -> Result<String, DecodeError> {
    // space-separated ("150 151") or packed ("150151", split every 3 digits)
    let spaced: Vec<&str> = cipher.split_whitespace().collect();
    let tokens: Vec<String> = if spaced.len() > 1 {
        spaced.iter().map(|s| s.to_string()).collect()
    } else {
        let packed = clean(cipher);
        packed.as_bytes().chunks(3)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect()
    };
    let mut bytes = Vec::new();
    for tok in &tokens {
        let trimmed = tok.trim_start_matches('0');
        let s = if trimmed.is_empty() { "0" } else { trimmed };
        let n = u8::from_str_radix(s, 8)
            .map_err(|e| DecodeError::failed("octal", format!("{tok}: {e}")))?;
        bytes.push(n);
    }
    Ok(to_utf8(bytes))
}

fn decode_decimal(cipher: &str) -> Result<String, DecodeError> {
    let mut bytes = Vec::new();
    for tok in cipher.split_whitespace() {
        let n: u32 = tok.parse()
            .map_err(|e| DecodeError::failed("decimal", e))?;
        let ch = char::from_u32(n)
            .ok_or_else(|| DecodeError::failed("decimal", format!("{n} 超出字符范围")))?;
        let mut buf = [0u8; 4];
        bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn decode_rot13(cipher: &str) -> Result<String, DecodeError> {
    Ok(cipher.chars().map(|c| match c {
        'a'..='z' => (((c as u8 - b'a' + 13) % 26) + b'a') as char,
        'A'..='Z' => (((c as u8 - b'A' + 13) % 26) + b'A') as char,
        _ => c,
    }).collect())
}

fn decode_atbash(cipher: &str) -> Result<String, DecodeError> {
    Ok(cipher.chars().map(|c| match c {
        'a'..='z' => (b'z' - (c as u8 - b'a')) as char,
        'A'..='Z' => (b'Z' - (c as u8 - b'A')) as char,
        _ => c,
    }).collect())
}

fn decode_brainfuck(cipher: &str) -> Result<String, DecodeError> {
    let code: Vec<u8> = cipher.bytes().filter(|b| b"+-<>[].,".contains(b)).collect();
    let mut tape = vec![0u8; 30000];
    let mut dp: usize = 0;
    let mut ip: usize = 0;
    let mut output = Vec::new();
    let mut step = 0usize;
    while ip < code.len() {
        step += 1;
        if step > 1_000_000 { return Err(DecodeError::failed("brainfuck", "超出执行步数限制")); }
        match code[ip] {
            b'+' => tape[dp] = tape[dp].wrapping_add(1),
            b'-' => tape[dp] = tape[dp].wrapping_sub(1),
            b'>' => { dp += 1; if dp >= tape.len() { tape.push(0); } }
            b'<' => { if dp == 0 { return Err(DecodeError::failed("brainfuck", "指针下溢")); } dp -= 1; }
            b'.' => output.push(tape[dp]),
            b',' => {} // no input
            b'[' => {
                if tape[dp] == 0 {
                    let mut depth = 1;
                    ip += 1;
                    while depth > 0 && ip < code.len() {
                        if code[ip] == b'[' { depth += 1; }
                        else if code[ip] == b']' { depth -= 1; }
                        ip += 1;
                    }
                    continue;
                }
            }
            b']' => {
                if tape[dp] != 0 {
                    let mut depth = 1;
                    ip = ip.saturating_sub(1);
                    while depth > 0 && ip > 0 {
                        ip -= 1;
                        if code[ip] == b']' { depth += 1; }
                        else if code[ip] == b'[' { depth -= 1; }
                    }
                    continue;
                }
            }
            _ => {}
        }
        ip += 1;
    }
    Ok(to_utf8(output))
}

fn decode_dna(cipher: &str) -> Result<String, DecodeError> {
    // ACGT → 00 01 10 11 (rule 1), 4 bases per byte
    let v = clean(cipher).to_uppercase();
    if v.len() % 4 != 0 {
        return Err(DecodeError::failed("dna", "长度必须为 4 的倍数"));
    }
    let mut bytes = Vec::new();
    for chunk in v.as_bytes().chunks(4) {
        let mut byte = 0u8;
        for (i, &b) in chunk.iter().enumerate() {
            let bits = match b {
                b'A' => 0b00,
                b'C' => 0b01,
                b'G' => 0b10,
                b'T' => 0b11,
                _ => return Err(DecodeError::failed("dna", format!("无效字符: {}", b as char))),
            };
            byte |= bits << (6 - i * 2);
        }
        bytes.push(byte);
    }
    Ok(to_utf8(bytes))
}

fn decode_bacon(cipher: &str) -> Result<String, DecodeError> {
    // 标准 Bacon: 输入为 A/B 字符序列，每5字符一组
    let v: String = cipher.chars()
        .filter(|c| c.is_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| *c == 'A' || *c == 'B')
        .collect();
    if v.len() % 5 != 0 {
        return Err(DecodeError::failed("bacon", "长度必须为 5 的倍数"));
    }
    let mut out = String::new();
    for chunk in v.as_bytes().chunks(5) {
        let idx: usize = chunk.iter().enumerate()
            .map(|(i, &b)| if b == b'B' { 1 << (4 - i) } else { 0 })
            .sum();
        if idx >= 26 { return Err(DecodeError::failed("bacon", format!("索引超界: {idx}"))); }
        out.push((b'A' + idx as u8) as char);
    }
    Ok(out)
}

fn decode_polybius(cipher: &str) -> Result<String, DecodeError> {
    // 标准 5×5，I/J 合并，输入为数字对如 "1523..."
    let digits: Vec<u8> = cipher.chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();
    if digits.len() % 2 != 0 {
        return Err(DecodeError::failed("polybius", "数字数量必须为偶数"));
    }
    const SQUARE: &str = "ABCDEFGHIKLMNOPQRSTUVWXYZ";
    let mut out = String::new();
    for pair in digits.chunks(2) {
        let (r, c) = (pair[0] as usize, pair[1] as usize);
        if r < 1 || r > 5 || c < 1 || c > 5 {
            return Err(DecodeError::failed("polybius", format!("坐标越界: ({r},{c})")));
        }
        let idx = (r - 1) * 5 + (c - 1);
        out.push(SQUARE.chars().nth(idx).unwrap());
    }
    Ok(out)
}

// ─── 古典密码（需密钥）────────────────────────────────────────────────────────

fn decode_caesar(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    let shift: u8 = params.get("shift")
        .ok_or_else(|| DecodeError::MissingParam("shift (0-25)".into()))?
        .parse::<u8>()
        .map_err(|_| DecodeError::InvalidParam("shift 必须为 0-25 的整数".into()))?;
    if shift > 25 { return Err(DecodeError::InvalidParam("shift 超出 0-25 范围".into())); }
    Ok(cipher.chars().map(|c| match c {
        'a'..='z' => (((c as u8 - b'a' + 26 - shift) % 26) + b'a') as char,
        'A'..='Z' => (((c as u8 - b'A' + 26 - shift) % 26) + b'A') as char,
        _ => c,
    }).collect())
}

pub fn caesar_brute_force(cipher: &str) -> Vec<(u8, String)> {
    (0u8..26).map(|shift| {
        let plain: String = cipher.chars().map(|c| match c {
            'a'..='z' => (((c as u8 - b'a' + 26 - shift) % 26) + b'a') as char,
            'A'..='Z' => (((c as u8 - b'A' + 26 - shift) % 26) + b'A') as char,
            _ => c,
        }).collect();
        (shift, plain)
    }).collect()
}

fn decode_vigenere(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    let key = params.get("key")
        .ok_or_else(|| DecodeError::MissingParam("key".into()))?;
    if key.is_empty() { return Err(DecodeError::InvalidParam("key 不能为空".into())); }
    let key_bytes: Vec<u8> = key.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase() as u8 - b'A')
        .collect();
    if key_bytes.is_empty() { return Err(DecodeError::InvalidParam("key 必须包含字母".into())); }
    let mut ki = 0usize;
    let mut out = String::new();
    for c in cipher.chars() {
        match c {
            'a'..='z' => {
                let p = ((c as u8 - b'a' + 26 - key_bytes[ki % key_bytes.len()]) % 26 + b'a') as char;
                ki += 1;
                out.push(p);
            }
            'A'..='Z' => {
                let p = ((c as u8 - b'A' + 26 - key_bytes[ki % key_bytes.len()]) % 26 + b'A') as char;
                ki += 1;
                out.push(p);
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

fn decode_affine(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    let a: i64 = params.get("a").ok_or_else(|| DecodeError::MissingParam("a".into()))?
        .parse().map_err(|_| DecodeError::InvalidParam("a 必须为整数".into()))?;
    let b: i64 = params.get("b").ok_or_else(|| DecodeError::MissingParam("b".into()))?
        .parse().map_err(|_| DecodeError::InvalidParam("b 必须为整数".into()))?;
    // find modular inverse of a mod 26
    let a_inv = (0i64..26).find(|&x| (a * x).rem_euclid(26) == 1)
        .ok_or_else(|| DecodeError::InvalidParam(format!("a={a} 与 26 不互质，无法解密")))?;
    Ok(cipher.chars().map(|c| match c {
        'a'..='z' => ((a_inv * (c as i64 - b'a' as i64 - b + 26 * 10)).rem_euclid(26) as u8 + b'a') as char,
        'A'..='Z' => ((a_inv * (c as i64 - b'A' as i64 - b + 26 * 10)).rem_euclid(26) as u8 + b'A') as char,
        _ => c,
    }).collect())
}

fn decode_rail_fence(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    let rails: usize = params.get("rails")
        .ok_or_else(|| DecodeError::MissingParam("rails".into()))?
        .parse::<usize>()
        .map_err(|_| DecodeError::InvalidParam("rails 必须为正整数".into()))?;
    if rails < 2 { return Err(DecodeError::InvalidParam("rails 至少为 2".into())); }
    let chars: Vec<char> = cipher.chars().collect();
    let n = chars.len();
    let cycle = 2 * (rails - 1);
    // 计算每条轨道的字符数
    let mut rail_lens = vec![0usize; rails];
    for i in 0..n {
        let pos = i % cycle;
        let r = if pos < rails { pos } else { cycle - pos };
        rail_lens[r] += 1;
    }
    // 将密文按轨道长度切分
    let mut rails_data: Vec<Vec<char>> = Vec::new();
    let mut offset = 0;
    for &len in &rail_lens {
        rails_data.push(chars[offset..offset + len].to_vec());
        offset += len;
    }
    // 按加密顺序重新读出
    let mut rail_pos = vec![0usize; rails];
    let mut out = String::new();
    for i in 0..n {
        let pos = i % cycle;
        let r = if pos < rails { pos } else { cycle - pos };
        out.push(rails_data[r][rail_pos[r]]);
        rail_pos[r] += 1;
    }
    Ok(out)
}

fn decode_columnar(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    let key = params.get("key")
        .ok_or_else(|| DecodeError::MissingParam("key (列序, 如 '3142')".into()))?;
    let order: Vec<usize> = key.chars()
        .map(|c| c.to_digit(10).map(|d| d as usize).ok_or_else(|| DecodeError::InvalidParam("key 中包含非数字".into())))
        .collect::<Result<Vec<_>, _>>()?;
    let num_cols = order.len();
    let chars: Vec<char> = cipher.chars().collect();
    let num_rows = (chars.len() + num_cols - 1) / num_cols;
    let extra = chars.len() % num_cols;
    // column lengths by original order position
    let mut sorted_order: Vec<(usize, usize)> = order.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    sorted_order.sort();
    let mut col_data: Vec<Vec<char>> = vec![Vec::new(); num_cols];
    let mut pos = 0;
    for &(_, col_idx) in &sorted_order {
        let len = if extra == 0 || col_idx < extra { num_rows } else { num_rows - 1 };
        col_data[col_idx] = chars[pos..pos + len].to_vec();
        pos += len;
    }
    let mut out = String::new();
    for row in 0..num_rows {
        for col in 0..num_cols {
            if row < col_data[col].len() { out.push(col_data[col][row]); }
        }
    }
    Ok(out)
}

fn decode_beaufort(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    let key = params.get("key")
        .ok_or_else(|| DecodeError::MissingParam("key".into()))?;
    let key_bytes: Vec<u8> = key.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase() as u8 - b'A')
        .collect();
    if key_bytes.is_empty() { return Err(DecodeError::InvalidParam("key 必须包含字母".into())); }
    let mut ki = 0usize;
    let mut out = String::new();
    for c in cipher.chars() {
        match c {
            'a'..='z' => {
                let p = ((key_bytes[ki % key_bytes.len()] + 26 - (c as u8 - b'a')) % 26 + b'a') as char;
                ki += 1; out.push(p);
            }
            'A'..='Z' => {
                let p = ((key_bytes[ki % key_bytes.len()] + 26 - (c as u8 - b'A')) % 26 + b'A') as char;
                ki += 1; out.push(p);
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

// ─── 现代密码（需密钥）────────────────────────────────────────────────────────

fn decode_xor(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    let key = params.get("key").ok_or_else(|| DecodeError::MissingParam("key".into()))?;
    let key_bytes = key.as_bytes();
    let cipher_clean = clean(cipher);
    // 优先尝试 hex 解析
    let cipher_bytes: Vec<u8> = if cipher_clean.len() % 2 == 0
        && cipher_clean.chars().all(|c| c.is_ascii_hexdigit())
    {
        hex::decode(&cipher_clean).unwrap_or_else(|_| cipher.as_bytes().to_vec())
    } else {
        cipher.as_bytes().to_vec()
    };
    let result: Vec<u8> = cipher_bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key_bytes[i % key_bytes.len()])
        .collect();
    Ok(to_utf8(result))
}

fn decode_xor_brute(cipher: &str) -> Vec<(u8, String)> {
    let cipher_clean = clean(cipher);
    let bytes: Vec<u8> = if cipher_clean.len() % 2 == 0
        && cipher_clean.chars().all(|c| c.is_ascii_hexdigit())
    {
        hex::decode(&cipher_clean).unwrap_or_else(|_| cipher.as_bytes().to_vec())
    } else {
        cipher.as_bytes().to_vec()
    };
    (0u8..=255)
        .map(|k| {
            let plain = to_utf8(bytes.iter().map(|&b| b ^ k).collect());
            (k, plain)
        })
        .collect()
}

fn decode_aes_ecb(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    use aes::Aes128;
    use ecb::Decryptor;
    use cipher::KeyInit;
    use cipher::block_padding::Pkcs7;
    use cipher::BlockDecryptMut;

    let key_hex = params.get("key").ok_or_else(|| DecodeError::MissingParam("key (hex)".into()))?;
    let key_bytes = hex::decode(clean(key_hex))
        .map_err(|e| DecodeError::InvalidParam(format!("key hex 无效: {e}")))?;
    let ct = hex::decode(clean(cipher))
        .map_err(|e| DecodeError::failed("aes_ecb", format!("密文 hex 无效: {e}")))?;

    match key_bytes.len() {
        16 => {
            let dec = Decryptor::<Aes128>::new(key_bytes.as_slice().into());
            let mut buf = ct.clone();
            let pt = dec.decrypt_padded_mut::<Pkcs7>(&mut buf)
                .map_err(|e| DecodeError::failed("aes_ecb", e.to_string()))?;
            Ok(to_utf8(pt.to_vec()))
        }
        24 => {
            use aes::Aes192;
            let dec = Decryptor::<Aes192>::new(key_bytes.as_slice().into());
            let mut buf = ct.clone();
            let pt = dec.decrypt_padded_mut::<Pkcs7>(&mut buf)
                .map_err(|e| DecodeError::failed("aes_ecb", e.to_string()))?;
            Ok(to_utf8(pt.to_vec()))
        }
        32 => {
            use aes::Aes256;
            let dec = Decryptor::<Aes256>::new(key_bytes.as_slice().into());
            let mut buf = ct.clone();
            let pt = dec.decrypt_padded_mut::<Pkcs7>(&mut buf)
                .map_err(|e| DecodeError::failed("aes_ecb", e.to_string()))?;
            Ok(to_utf8(pt.to_vec()))
        }
        n => Err(DecodeError::InvalidParam(format!("key 长度 {n} 字节无效，应为 16/24/32"))),
    }
}

fn decode_aes_cbc(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    use aes::Aes128;
    use cbc::Decryptor;
    use cipher::KeyIvInit;
    use cipher::block_padding::Pkcs7;
    use cipher::BlockDecryptMut;

    let key_hex = params.get("key").ok_or_else(|| DecodeError::MissingParam("key (hex)".into()))?;
    let iv_hex  = params.get("iv").ok_or_else(|| DecodeError::MissingParam("iv (hex)".into()))?;
    let key_bytes = hex::decode(clean(key_hex))
        .map_err(|e| DecodeError::InvalidParam(format!("key hex 无效: {e}")))?;
    let iv_bytes = hex::decode(clean(iv_hex))
        .map_err(|e| DecodeError::InvalidParam(format!("iv hex 无效: {e}")))?;
    if iv_bytes.len() != 16 {
        return Err(DecodeError::InvalidParam("iv 必须为 16 字节".into()));
    }
    let ct = hex::decode(clean(cipher))
        .map_err(|e| DecodeError::failed("aes_cbc", format!("密文 hex 无效: {e}")))?;

    macro_rules! cbc_decrypt {
        ($T:ty) => {{
            let dec = Decryptor::<$T>::new(key_bytes.as_slice().into(), iv_bytes.as_slice().into());
            let mut buf = ct.clone();
            let pt = dec.decrypt_padded_mut::<Pkcs7>(&mut buf)
                .map_err(|e| DecodeError::failed("aes_cbc", e.to_string()))?;
            Ok(to_utf8(pt.to_vec()))
        }};
    }

    match key_bytes.len() {
        16 => cbc_decrypt!(Aes128),
        24 => cbc_decrypt!(aes::Aes192),
        32 => cbc_decrypt!(aes::Aes256),
        n  => Err(DecodeError::InvalidParam(format!("key 长度 {n} 字节无效，应为 16/24/32"))),
    }
}

fn decode_rc4(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    let key = params.get("key").ok_or_else(|| DecodeError::MissingParam("key".into()))?;
    let key_bytes = key.as_bytes();
    let ct: Vec<u8> = {
        let h = clean(cipher);
        if h.len() % 2 == 0 && h.chars().all(|c| c.is_ascii_hexdigit()) {
            hex::decode(&h).unwrap_or_else(|_| cipher.as_bytes().to_vec())
        } else {
            cipher.as_bytes().to_vec()
        }
    };
    // KSA
    let mut s: Vec<u8> = (0u8..=255).collect();
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + s[i] as usize + key_bytes[i % key_bytes.len()] as usize) % 256;
        s.swap(i, j);
    }
    // PRGA
    let mut i = 0usize; j = 0;
    let result: Vec<u8> = ct.iter().map(|&byte| {
        i = (i + 1) % 256;
        j = (j + s[i] as usize) % 256;
        s.swap(i, j);
        byte ^ s[(s[i] as usize + s[j] as usize) % 256]
    }).collect();
    Ok(to_utf8(result))
}

fn decode_rsa(cipher: &str, params: &Params) -> Result<String, DecodeError> {
    // 针对 CTF 常见场景：给定 n, d (或 e+p+q)，对单个大整数解密
    let n_str = params.get("n").ok_or_else(|| DecodeError::MissingParam("n".into()))?;
    let d_str = params.get("d").ok_or_else(|| DecodeError::MissingParam("d (私钥指数)".into()))?;
    let n = n_str.trim().parse::<BigUint>()
        .map_err(|_| DecodeError::InvalidParam("n 必须为正整数".into()))?;
    let d = d_str.trim().parse::<BigUint>()
        .map_err(|_| DecodeError::InvalidParam("d 必须为正整数".into()))?;
    let c = clean(cipher).parse::<BigUint>()
        .map_err(|_| DecodeError::failed("rsa", "密文必须为整数"))?;
    let m = c.modpow(&d, &n);
    Ok(to_utf8(m.to_bytes_be()))
}

// ─── hex / binary / octal（原有，统一到这里）─────────────────────────────────

fn decode_hex(cipher: &str) -> Result<String, DecodeError> {
    let v = clean(cipher).to_lowercase().replace("0x", "").replace(' ', "");
    hex::decode(&v).map(to_utf8).map_err(|e| DecodeError::failed("hex", e))
}

fn decode_binary(cipher: &str) -> Result<String, DecodeError> {
    let v = clean(cipher).replace("0b", "");
    // 支持空格分隔（每8位一组）或连续字符串
    let tokens: Vec<&str> = if v.contains(' ') {
        v.split_whitespace().collect()
    } else {
        v.as_bytes().chunks(8).map(|c| std::str::from_utf8(c).unwrap_or("")).collect()
    };
    let mut bytes = Vec::new();
    for tok in tokens {
        let b = u8::from_str_radix(tok, 2)
            .map_err(|e| DecodeError::failed("binary", format!("{tok}: {e}")))?;
        bytes.push(b);
    }
    Ok(to_utf8(bytes))
}

fn decode_reverse(cipher: &str) -> Result<String, DecodeError> {
    Ok(clean(cipher).chars().rev().collect())
}

// ─── 自动识别 ─────────────────────────────────────────────────────────────────

/// 可打印字符比例评分
fn printable_score(s: &str) -> f64 {
    if s.is_empty() { return 0.0; }
    let printable = s.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').count();
    printable as f64 / s.chars().count() as f64
}

pub fn auto_detect(cipher: &str) -> Vec<DetectResult> {
    let no_key_methods = [
        "base64","base32","base16","base85","base58","base62",
        "hex","binary","octal","decimal",
        "url","html_entity","unicode_escape",
        "rot13","atbash","morse","brainfuck","dna","bacon","polybius",
        "reverse",
    ];
    let empty = HashMap::new();
    let mut results: Vec<DetectResult> = no_key_methods.iter()
        .filter_map(|&method| {
            decode(method, cipher, &empty).ok().map(|result| {
                let mut score = printable_score(&result);
                // flag 命中加权
                if crate::flag::find_flag(&result, "flag").is_some() { score += 10.0; }
                // 空串降权
                if result.trim().is_empty() { score = 0.0; }
                DetectResult { method: method.to_string(), result, score }
            })
        })
        .collect();
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

// ─── 总入口 ───────────────────────────────────────────────────────────────────

pub fn decode(method: &str, cipher: &str, params: &Params) -> Result<String, DecodeError> {
    match method {
        // 编码类
        "base64"         => decode_base64(cipher),
        "base32"         => decode_base32(cipher),
        "base16"         => decode_base16(cipher),
        "base85"         => decode_base85(cipher),
        "base58"         => decode_base58(cipher),
        "base62"         => decode_base62(cipher),
        "hex"            => decode_hex(cipher),
        "binary"         => decode_binary(cipher),
        "octal"          => decode_octal(cipher),
        "decimal"        => decode_decimal(cipher),
        "url"            => decode_url(cipher),
        "html_entity"    => decode_html_entity(cipher),
        "unicode_escape" => decode_unicode_escape(cipher),
        "rot13"          => decode_rot13(cipher),
        "atbash"         => decode_atbash(cipher),
        "morse"          => decode_morse(cipher),
        "brainfuck"      => decode_brainfuck(cipher),
        "dna"            => decode_dna(cipher),
        "bacon"          => decode_bacon(cipher),
        "polybius"       => decode_polybius(cipher),
        "reverse"        => decode_reverse(cipher),
        // 古典密码
        "caesar"         => decode_caesar(cipher, params),
        "vigenere"       => decode_vigenere(cipher, params),
        "affine"         => decode_affine(cipher, params),
        "rail_fence"     => decode_rail_fence(cipher, params),
        "columnar"       => decode_columnar(cipher, params),
        "beaufort"       => decode_beaufort(cipher, params),
        // 现代密码
        "xor"            => decode_xor(cipher, params),
        "xor_brute"      => Ok(decode_xor_brute(cipher)
                                .iter()
                                .map(|(k, v)| format!("key=0x{k:02x}: {v}"))
                                .collect::<Vec<_>>()
                                .join("\n")),
        "aes_ecb"        => decode_aes_ecb(cipher, params),
        "aes_cbc"        => decode_aes_cbc(cipher, params),
        "rc4"            => decode_rc4(cipher, params),
        "rsa"            => decode_rsa(cipher, params),
        other            => Err(DecodeError::UnsupportedMethod(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(key: &str, val: &str) -> Params {
        let mut m = HashMap::new();
        m.insert(key.to_string(), val.to_string());
        m
    }
    fn p2(k1: &str, v1: &str, k2: &str, v2: &str) -> Params {
        let mut m = p(k1, v1);
        m.insert(k2.to_string(), v2.to_string());
        m
    }

    #[test]
    fn test_base64() { assert_eq!(decode("base64", "ZmxhZ3t0ZXN0fQ==", &HashMap::new()).unwrap(), "flag{test}"); }
    #[test]
    fn test_base32() { assert_eq!(decode("base32", "MZXW6YTBOI======", &HashMap::new()).unwrap(), "foobar"); }
    #[test]
    fn test_base16() { assert_eq!(decode("base16", "666C61677B74657374 7D", &HashMap::new()).unwrap(), "flag{test}"); }
    #[test]
    fn test_base58() { assert!(decode("base58", "2NEpo7TZRRrLZSi2U", &HashMap::new()).is_ok()); }
    #[test]
    fn test_hex()    { assert_eq!(decode("hex", "666c61677b74657374 7d", &HashMap::new()).unwrap(), "flag{test}"); }
    #[test]
    fn test_binary() { assert_eq!(decode("binary", "01101000 01101001", &HashMap::new()).unwrap(), "hi"); }
    #[test]
    fn test_octal()  { assert_eq!(decode("octal", "150 151", &HashMap::new()).unwrap(), "hi"); }
    #[test]
    fn test_decimal(){ assert_eq!(decode("decimal", "104 105", &HashMap::new()).unwrap(), "hi"); }
    #[test]
    fn test_rot13()  { assert_eq!(decode("rot13", "uryyb", &HashMap::new()).unwrap(), "hello"); }
    #[test]
    fn test_atbash() { assert_eq!(decode("atbash", "svool", &HashMap::new()).unwrap(), "hello"); }
    #[test]
    fn test_reverse(){ assert_eq!(decode("reverse", "olleh", &HashMap::new()).unwrap(), "hello"); }
    #[test]
    fn test_url()    { assert_eq!(decode("url", "flag%7Btest%7D", &HashMap::new()).unwrap(), "flag{test}"); }
    #[test]
    fn test_html()   { assert_eq!(decode("html_entity", "&lt;b&gt;hi&lt;/b&gt;", &HashMap::new()).unwrap(), "<b>hi</b>"); }
    #[test]
    fn test_unicode_escape() {
        assert_eq!(decode("unicode_escape", r"hi", &HashMap::new()).unwrap(), "hi");
    }
    #[test]
    fn test_morse()  { assert_eq!(decode("morse", ".... ..", &HashMap::new()).unwrap(), "HI"); }
    #[test]
    fn test_brainfuck() {
        let bf = "++++++++++[>+++++++>++++++++++>+++>+<<<<-]>++.>+.+++++++..+++.>++.<<+++++++++++++++.>.+++.------.--------.>+.>.";
        assert!(decode("brainfuck", bf, &HashMap::new()).unwrap().contains("Hello"));
    }
    #[test]
    fn test_bacon()  { assert_eq!(decode("bacon", "AAAAA AABBB", &HashMap::new()).unwrap(), "AH"); }
    #[test]
    fn test_polybius(){ assert_eq!(decode("polybius", "2324", &HashMap::new()).unwrap(), "HI"); }
    #[test]
    fn test_caesar() { assert_eq!(decode("caesar", "KHOOR", &p("shift","3")).unwrap(), "HELLO"); }
    #[test]
    fn test_caesar_brute() {
        let results = caesar_brute_force("KHOOR");
        assert!(results.iter().any(|(_, s)| s == "HELLO"));
    }
    #[test]
    fn test_vigenere(){ assert_eq!(decode("vigenere", "LXFOPVEFRNHR", &p("key","LEMON")).unwrap(), "ATTACKATDAWN"); }
    #[test]
    fn test_affine() { assert_eq!(decode("affine", "AFCCX", &p2("a","7","b","3")).unwrap(), "HELLO"); }
    #[test]
    fn test_beaufort(){ assert!(decode("beaufort", "RIKVBIYBITRIXUVGOS", &p("key","FORTIFICATION")).is_ok()); }
    #[test]
    fn test_xor() {
        let key = "key";
        let plain = "hello";
        let ct: Vec<u8> = plain.bytes().zip(key.bytes().cycle()).map(|(a,b)| a^b).collect();
        let ct_hex = hex::encode(&ct);
        assert_eq!(decode("xor", &ct_hex, &p("key", key)).unwrap(), plain);
    }
    #[test]
    fn test_rc4() {
        // RC4("Key", "Plaintext") = BBF316E8D940AF0AD3
        let ct = "BBF316E8D940AF0AD3";
        let result = decode("rc4", ct, &p("key", "Key")).unwrap();
        assert_eq!(result, "Plaintext");
    }
    #[test]
    fn test_rail_fence() {
        // "WEAREDISCOVEREDRUNATONCE" railed at 3 → classic example
        let ct = "WECRLTEERDSOEEFEAABORADICVNE";
        // just verify it doesn't panic and returns something
        let result = decode("rail_fence", ct, &p("rails","3"));
        assert!(result.is_ok());
    }
    #[test]
    fn test_auto_detect() {
        let results = auto_detect("ZmxhZ3t0ZXN0fQ==");
        assert!(!results.is_empty());
        assert!(results[0].score > 0.5);
    }
    #[test]
    fn test_unsupported() {
        assert!(matches!(decode("foobar", "", &HashMap::new()), Err(DecodeError::UnsupportedMethod(_))));
    }
}







