// Method registry — single source of truth for sidebar + param rendering
// FIXME: 与后端 decode() 的 match 分支是双份手写维护，新增方法需两边同步；
// 可考虑由后端 methods_cmd 返回元数据，前端直接渲染
export interface Param {
  key: string;
  label: string;
  placeholder?: string;
  type?: "text" | "number";
}

export interface Method {
  id: string;
  label: string;
  needsKey?: boolean;
  params?: Param[];
}

export interface Group {
  label: string;
  methods: Method[];
}

export const GROUPS: Group[] = [
  {
    label: "编码类",
    methods: [
      { id: "base64",         label: "Base64" },
      { id: "base32",         label: "Base32" },
      { id: "base16",         label: "Base16 (Hex)" },
      { id: "base85",         label: "Base85 / ASCII85" },
      { id: "base58",         label: "Base58" },
      { id: "base62",         label: "Base62" },
      { id: "hex",            label: "Hex" },
      { id: "binary",         label: "Binary" },
      { id: "octal",          label: "Octal" },
      { id: "decimal",        label: "Decimal (ASCII)" },
      { id: "url",            label: "URL Encode" },
      { id: "html_entity",    label: "HTML Entity" },
      { id: "unicode_escape", label: "Unicode Escape" },
      { id: "morse",          label: "Morse Code" },
      { id: "dna",            label: "DNA 编码" },
      { id: "bacon",          label: "培根密码" },
      { id: "polybius",       label: "波利比奥斯方阵" },
      { id: "brainfuck",      label: "Brainfuck" },
      { id: "rot13",          label: "ROT13" },
      { id: "atbash",         label: "Atbash" },
      { id: "reverse",        label: "反转字符串" },
    ],
  },
  {
    label: "古典密码",
    methods: [
      {
        id: "caesar", label: "Caesar 移位",
        params: [{ key: "shift", label: "偏移量", placeholder: "0–25", type: "number" }],
      },
      { id: "caesar_brute", label: "Caesar 暴力枚举" },
      {
        id: "vigenere", label: "Vigenère",
        params: [{ key: "key", label: "密钥", placeholder: "如 LEMON" }],
      },
      {
        id: "affine", label: "Affine",
        params: [
          { key: "a", label: "a (与 26 互质)", placeholder: "如 7", type: "number" },
          { key: "b", label: "b", placeholder: "如 3",  type: "number" },
        ],
      },
      {
        id: "rail_fence", label: "栏栅密码",
        params: [{ key: "rails", label: "栏数", placeholder: "如 3", type: "number" }],
      },
      {
        id: "columnar", label: "列置换",
        params: [{ key: "key", label: "列序", placeholder: "如 3142" }],
      },
      {
        id: "beaufort", label: "Beaufort",
        params: [{ key: "key", label: "密钥", placeholder: "如 FORTIFICATION" }],
      },
    ],
  },
  {
    label: "现代密码",
    methods: [
      {
        id: "xor", label: "XOR",
        params: [{ key: "key", label: "密钥 (字符串或 hex)" }],
      },
      { id: "xor_brute", label: "XOR 单字节暴力" },
      {
        id: "aes_ecb", label: "AES-ECB",
        params: [{ key: "key", label: "Key (hex)", placeholder: "32/48/64 位 hex" }],
      },
      {
        id: "aes_cbc", label: "AES-CBC",
        params: [
          { key: "key", label: "Key (hex)", placeholder: "32/48/64 位 hex" },
          { key: "iv",  label: "IV (hex)",  placeholder: "32 位 hex" },
        ],
      },
      {
        id: "rc4", label: "RC4",
        params: [{ key: "key", label: "密钥" }],
      },
      {
        id: "rsa", label: "RSA 解密",
        params: [
          { key: "n", label: "n (模数)" },
          { key: "d", label: "d (私钥指数)" },
        ],
      },
    ],
  },
  {
    label: "工具",
    methods: [
      { id: "auto_detect", label: "自动识别" },
      { id: "find_flag",   label: "寻找 Flag" },
    ],
  },
];

// Flat lookup
export const METHOD_MAP = new Map<string, Method>(
  GROUPS.flatMap((g) => g.methods.map((m) => [m.id, m]))
);
