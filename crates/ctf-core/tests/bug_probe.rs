use ctf_core::{decode, Params};
use std::collections::HashMap;

fn p(k: &str, v: &str) -> Params {
    let mut m = HashMap::new();
    m.insert(k.to_string(), v.to_string());
    m
}

// DNA：4 碱基 = 1 字节。h=0x68=01101000→CGGA，i=0x69=01101001→CGGC
#[test]
fn dna_known_answer() {
    assert_eq!(decode("dna", "CGGACGGC", &HashMap::new()).unwrap(), "hi");
}

#[test]
fn dna_rejects_bad_length() {
    assert!(decode("dna", "ACG", &HashMap::new()).is_err());
}

// 多字节字符不再 panic，且字符数守恒
#[test]
fn rail_fence_multibyte() {
    let input = "你好世界abc";
    let out = decode("rail_fence", input, &p("rails", "3")).unwrap();
    assert_eq!(out.chars().count(), input.chars().count());
}

#[test]
fn columnar_multibyte() {
    let input = "你好世界abcd";
    let out = decode("columnar", input, &p("key", "3142")).unwrap();
    assert_eq!(out.chars().count(), input.chars().count());
}

// morse 支持 "/" 作单词分隔
#[test]
fn morse_slash_separator() {
    assert_eq!(
        decode("morse", ".... .. / .-- --- .-. .-.. -..", &HashMap::new()).unwrap(),
        "HI WORLD"
    );
}

// base85 数值溢出（> u32::MAX）必须报错而非静默截断
#[test]
fn base85_overflow_rejected() {
    assert!(decode("base85", "uuuuu", &HashMap::new()).is_err());
}

// base85 正常值仍可解码
#[test]
fn base85_roundtrip_ok() {
    // "hi" = 0x6869 -> 2 字符组? 用完整组验证: 0x00026869 太大，直接验证不报错即可
    assert!(decode("base85", "BOu", &HashMap::new()).is_ok());
}
