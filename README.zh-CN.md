# CTF Tool

[English](README.md) | [简体中文](README.zh-CN.md)

CTF 比赛用解码 / 解密桌面工具。Rust workspace + Tauri 2 + React，粘贴即出结果，支持多层链式解码与 flag 自动识别。

## 功能

- **编码类（21 种）**：Base64/32/16/85/58/62、Hex、Binary、Octal、Decimal、URL、HTML 实体、Unicode 转义、Morse、Brainfuck、DNA、培根密码、波利比奥斯方阵、ROT13、Atbash、反转
- **古典密码**：Caesar（含暴力枚举）、Vigenère、Affine、栏栅、列置换、Beaufort
- **现代密码**：XOR（含单字节暴力）、AES-128/192/256（ECB/CBC）、RC4、RSA（n + d）
- **工具**：
  - 自动识别——对所有无密钥方法跑一遍，按可打印比例 + flag 命中打分排序
  - 寻找 Flag——自定义 flag 头，全文高亮、一键复制
  - 链式解码——"作为输入"把结果回灌，对付多层嵌套编码

解码对无效字节采用 `errors=ignore` 语义（静默丢弃非法 UTF-8）。

## 快速开始

```bash
cd frontend
npm install
npm run app        # 开发模式（Vite 热更新 + Tauri 窗口）
```

打包 release 可执行文件：

```bash
cd frontend
npx tauri build --no-bundle   # 产物在 target/release/ctf-app.exe
# 去掉 --no-bundle 可生成 MSI/NSIS 安装包（需 WiX/NSIS）
```

## 项目结构

```
crates/ctf-core/      核心解码库（纯 Rust，33 种方法 + 自动识别 + flag 提取）
crates/ctf-gui/       Tauri 壳（FIXME: 与 frontend/src-tauri 重复，待删）
frontend/             React + Vite 前端
frontend/src-tauri/   Tauri 壳（实际使用，包名 ctf-app）
```

## 测试

```bash
cargo test -p ctf-core    # 35 个测试（含边界回归）
```

## 环境要求

- Rust（MSVC 工具链）+ Visual Studio 2022（C++ 生成工具）
- Node.js 18+
- 注意：若环境变量 `CC`/`CXX` 指向 MinGW gcc，会导致 MSVC 链接失败（LNK1143），请先清除
