import { invoke } from "@tauri-apps/api/core";

export interface DetectResult {
  method: string;
  result: string;
  score: number;
}

export async function decode(
  method: string,
  cipher: string,
  params: Record<string, string> = {}
): Promise<string> {
  return invoke<string>("decode_cmd", { req: { method, cipher, params } });
}

export async function findFlag(text: string, header: string): Promise<string | null> {
  return invoke<string | null>("find_flag_cmd", { text, header });
}

export async function caesarBrute(cipher: string): Promise<[number, string][]> {
  return invoke<[number, string][]>("caesar_brute_cmd", { cipher });
}

export async function autoDetect(cipher: string): Promise<DetectResult[]> {
  return invoke<DetectResult[]>("auto_detect_cmd", { cipher });
}
