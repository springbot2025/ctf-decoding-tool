import { useState, useEffect, useMemo, useRef, useCallback, type ReactNode } from "react";
import { GROUPS, METHOD_MAP, type Method } from "./methods";
import { decode, findFlag, caesarBrute, autoDetect, type DetectResult } from "./api";

type Mode = "decode" | "caesar_brute" | "xor_brute" | "auto_detect" | "find_flag";
interface Row { key: string; text: string; isFlag: boolean; }

const SPECIAL_MODES: Mode[] = ["caesar_brute", "xor_brute", "auto_detect", "find_flag"];
const modeOf = (id: string): Mode =>
  (SPECIAL_MODES as string[]).includes(id) ? (id as Mode) : "decode";

const LS_METHOD = "ctf.method";
const LS_HEADER = "ctf.flagHeader";
const lsGet = (k: string, d: string) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
const lsSet = (k: string, v: string) => { try { localStorage.setItem(k, v); } catch { /* ignore */ } };

/** 与后端 normalize_header 保持一致：取 `{` 前的部分，空则用 flag */
function flagRegex(header: string, global = false): RegExp {
  const base = header.trim().split("{")[0] || "flag";
  const esc = base.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`${esc}\\{[^{}]+\\}`, global ? "gi" : "i");
}

function findAllFlags(text: string, header: string): string[] {
  return Array.from(new Set(text.match(flagRegex(header, true)) ?? []));
}

/** 把文本中的 flag 用 <mark> 高亮 */
function highlight(text: string, header: string): ReactNode {
  const re = flagRegex(header, true);
  const out: ReactNode[] = [];
  let last = 0;
  for (const m of text.matchAll(re)) {
    const i = m.index ?? 0;
    if (i > last) out.push(text.slice(last, i));
    out.push(<mark key={i} className="flag-hl">{m[0]}</mark>);
    last = i + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setV(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return v;
}

// ─── Sidebar ───────────────────────────────────────────────────────────────
function Sidebar({
  active, onSelect, searchRef,
}: {
  active: string;
  onSelect: (id: string) => void;
  searchRef: React.RefObject<HTMLInputElement>;
}) {
  const [q, setQ] = useState("");
  const groups = useMemo(() => {
    const s = q.trim().toLowerCase();
    if (!s) return GROUPS;
    return GROUPS
      .map((g) => ({
        ...g,
        methods: g.methods.filter((m) =>
          m.label.toLowerCase().includes(s) || m.id.toLowerCase().includes(s)),
      }))
      .filter((g) => g.methods.length);
  }, [q]);

  const first = groups[0]?.methods[0];

  return (
    <nav className="sidebar" aria-label="解码方法">
      <div className="sidebar__search">
        <input
          ref={searchRef}
          className="input"
          placeholder="搜索方法…"
          aria-label="搜索方法"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && first) { onSelect(first.id); setQ(""); }
            if (e.key === "Escape") { setQ(""); e.currentTarget.blur(); }
          }}
        />
        <kbd>Ctrl K</kbd>
      </div>
      <div className="sidebar__list">
        {groups.map((g) => (
          <div key={g.label} role="group" aria-label={g.label}>
            <div className="sidebar__group-label">{g.label}</div>
            {g.methods.map((m) => (
              <button
                key={m.id}
                className={`sidebar__item${active === m.id ? " sidebar__item--active" : ""}`}
                aria-current={active === m.id ? "page" : undefined}
                onClick={() => onSelect(m.id)}
              >
                <span>{m.label}</span>
                {m.params?.length ? <span className="sidebar__item-tag">需参数</span> : null}
              </button>
            ))}
          </div>
        ))}
        {!groups.length && <div className="sidebar__empty">没有匹配「{q}」的方法</div>}
      </div>
    </nav>
  );
}

// ─── Small pieces ──────────────────────────────────────────────────────────
function Empty({ children }: { children: ReactNode }) {
  return <div className="empty">{children}</div>;
}

function FlagBanner({ flags, onCopy }: { flags: string[]; onCopy: (s: string) => void }) {
  if (!flags.length) return null;
  return (
    <div className="flag-banner" role="status">
      {flags.map((f) => (
        <button key={f} className="flag-chip" onClick={() => onCopy(f)} title="点击复制">
          🚩 {f}
        </button>
      ))}
    </div>
  );
}

function ParamsBar({
  method, params, onChange,
}: {
  method: Method;
  params: Record<string, string>;
  onChange: (key: string, value: string) => void;
}) {
  if (!method.params?.length) return null;
  return (
    <div className="params">
      {method.params.map((p) => (
        <label key={p.key} className="field-label">
          {p.label}
          <input
            className="input input--mono"
            type={p.type === "number" ? "number" : "text"}
            placeholder={p.placeholder ?? ""}
            value={params[p.key] ?? ""}
            onChange={(e) => onChange(p.key, e.target.value)}
            spellCheck={false}
          />
        </label>
      ))}
    </div>
  );
}

/** 暴力枚举结果表：点行复制，含 flag 的行高亮 */
function RowsTable({
  head, rows, onCopy, onlyFlags,
}: {
  head: string; rows: Row[]; onCopy: (s: string) => void; onlyFlags: boolean;
}) {
  const shown = onlyFlags ? rows.filter((r) => r.isFlag) : rows;
  if (!shown.length) return <Empty>{onlyFlags ? "没有包含 flag 的结果" : "无结果"}</Empty>;
  return (
    <table className="rows">
      <thead><tr><th>{head}</th><th>明文（点击复制）</th></tr></thead>
      <tbody>
        {shown.map((r) => (
          <tr key={r.key} className={r.isFlag ? "is-flag" : ""} onClick={() => onCopy(r.text)}>
            <td className="k">{r.key}</td>
            <td>{r.text}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function DetectTable({
  items, onCopy, onUse,
}: {
  items: DetectResult[]; onCopy: (s: string) => void; onUse: (s: string) => void;
}) {
  const max = Math.max(1, ...items.map((i) => i.score));
  return (
    <table className="rows">
      <thead><tr><th>方法</th><th>评分</th><th>结果（点击复制）</th><th /></tr></thead>
      <tbody>
        {items.map((r) => (
          <tr key={r.method} className={r.score >= 10 ? "is-flag" : ""} onClick={() => onCopy(r.result)}>
            <td className="k">{METHOD_MAP.get(r.method)?.label ?? r.method}</td>
            <td className="k">
              <span className="score">
                <span className="score__bar"><span style={{ width: `${(r.score / max) * 100}%` }} /></span>
                {r.score.toFixed(1)}
              </span>
            </td>
            <td>{r.result || <span style={{ opacity: 0.5 }}>（空）</span>}</td>
            <td className="k">
              <button
                className="btn"
                onClick={(e) => { e.stopPropagation(); onUse(r.result); }}
                title="把这条结果作为新的输入继续解码"
              >
                ↵ 继续
              </button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

// ─── Main App ──────────────────────────────────────────────────────────────
export default function App() {
  const [activeId, setActiveId] = useState(() => {
    const saved = lsGet(LS_METHOD, "base64");
    return METHOD_MAP.has(saved) ? saved : "base64";
  });
  const [cipher, setCipher] = useState("");
  const [params, setParams] = useState<Record<string, string>>({});
  const [flagHeader, setFlagHeader] = useState(() => lsGet(LS_HEADER, "flag"));

  const [result, setResult] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [rows, setRows] = useState<Row[]>([]);
  const [detect, setDetect] = useState<DetectResult[]>([]);
  const [found, setFound] = useState<string | null>(null);
  const [onlyFlags, setOnlyFlags] = useState(false);

  const [toast, setToast] = useState("");
  const toastTimer = useRef<number>();
  const searchRef = useRef<HTMLInputElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const method = METHOD_MAP.get(activeId)!;
  const mode = modeOf(activeId);

  const dCipher = useDebounced(cipher, 180);
  const dParams = useDebounced(params, 250);
  const dHeader = useDebounced(flagHeader, 250);
  const paramsKey = JSON.stringify(dParams);

  const notify = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(""), 1400);
  }, []);

  const copy = useCallback((text: string) => {
    if (!text) return;
    navigator.clipboard.writeText(text)
      .then(() => notify("已复制到剪贴板"))
      .catch(() => notify("复制失败"));
  }, [notify]);

  const select = useCallback((id: string) => {
    setActiveId(id);
    lsSet(LS_METHOD, id);
    setParams({});
    setError("");
    inputRef.current?.focus();
  }, []);

  /** 把输出作为新输入，便于多层嵌套解码 */
  const chainInput = useCallback((text: string) => {
    setCipher(text);
    inputRef.current?.focus();
    notify("已作为新输入");
  }, [notify]);

  useEffect(() => { lsSet(LS_HEADER, flagHeader); }, [flagHeader]);

  // 全局快捷键：Ctrl+K 搜索方法
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // 输入/方法/参数变化后重新计算
  useEffect(() => {
    setResult(""); setError(""); setRows([]); setDetect([]); setFound(null);
    if (!dCipher.trim()) { setLoading(false); return; }

    let cancelled = false;
    setLoading(true);
    (async () => {
      try {
        if (mode === "decode") {
          const r = await decode(activeId, dCipher, dParams);
          if (!cancelled) setResult(r);
        } else if (mode === "caesar_brute") {
          const list = await caesarBrute(dCipher);
          if (!cancelled) setRows(list.map(([k, p]) => ({
            key: `+${k}`, text: p, isFlag: flagRegex(dHeader).test(p),
          })));
        } else if (mode === "xor_brute") {
          const raw = await decode("xor_brute", dCipher, {});
          if (!cancelled) setRows(raw.split("\n").filter(Boolean).map((line) => {
            const [k, ...rest] = line.split(": ");
            const p = rest.join(": ");
            return { key: k, text: p, isFlag: flagRegex(dHeader).test(p) };
          }));
        } else if (mode === "auto_detect") {
          const items = await autoDetect(dCipher);
          if (!cancelled) setDetect(items.filter((i) => i.score > 0));
        } else if (mode === "find_flag") {
          const f = await findFlag(dCipher, dHeader || "flag");
          if (!cancelled) setFound(f ?? null);
        }
      } catch (e: unknown) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dCipher, activeId, paramsKey, dHeader]);

  const flags = useMemo(() => {
    if (mode === "decode") return findAllFlags(result, dHeader);
    if (mode === "find_flag") {
      const all = findAllFlags(dCipher, dHeader);
      return found && !all.includes(found) ? [found, ...all] : all;
    }
    return [];
  }, [mode, result, dCipher, dHeader, found]);

  const flagRowCount = rows.filter((r) => r.isFlag).length;

  const outputTitle =
    mode === "caesar_brute" ? "Caesar 枚举"
    : mode === "xor_brute"  ? "XOR 单字节枚举"
    : mode === "auto_detect" ? "自动识别"
    : mode === "find_flag"   ? "Flag 搜索"
    : "输出";

  const outputMeta =
    mode === "decode" && result ? `${result.length} 字符`
    : rows.length ? `${rows.length} 条 · ${flagRowCount} 含 flag`
    : detect.length ? `${detect.length} 种可能`
    : mode === "find_flag" && dCipher ? `${flags.length} 个匹配`
    : "";

  let body: ReactNode;
  if (error) {
    body = <div className="error" role="alert">{error}</div>;
  } else if (!dCipher.trim()) {
    body = <Empty>在左侧输入内容，结果会自动更新</Empty>;
  } else if (mode === "decode") {
    body = result
      ? <div className="output">{highlight(result, dHeader)}</div>
      : !loading && <Empty>结果为空</Empty>;
  } else if (mode === "caesar_brute" || mode === "xor_brute") {
    body = rows.length
      ? <RowsTable head={mode === "xor_brute" ? "Key" : "偏移"} rows={rows} onCopy={copy} onlyFlags={onlyFlags} />
      : !loading && <Empty>无结果</Empty>;
  } else if (mode === "auto_detect") {
    body = detect.length
      ? <DetectTable items={detect} onCopy={copy} onUse={chainInput} />
      : !loading && <Empty>没有识别出可能的编码</Empty>;
  } else {
    body = flags.length
      ? <div className="output">{highlight(dCipher, dHeader)}</div>
      : !loading && <Empty>未找到 {(dHeader.split("{")[0] || "flag")}{"{...}"} 格式的 flag</Empty>;
  }

  return (
    <div className="layout">
      <header className="topbar">
        <span className="topbar__logo" aria-hidden>{"{}"}</span>
        <span className="topbar__title">CTF Tool</span>
        <span className="topbar__badge">{method.label}</span>
        <span className="topbar__spacer" />
        <label className="field-label">
          Flag 头
          <input
            className="input input--mono"
            style={{ width: 96 }}
            placeholder="flag"
            value={flagHeader}
            onChange={(e) => setFlagHeader(e.target.value)}
            spellCheck={false}
          />
        </label>
      </header>

      <Sidebar active={activeId} onSelect={select} searchRef={searchRef} />

      <main className="main">
        <div className="pane">
          <section className="card" aria-label="输入">
            <div className="card__header">
              <span className="card__title">输入</span>
              <span className="card__meta">{cipher ? `${cipher.length} 字符` : ""}</span>
              <span className="card__spacer" />
              <button
                className="btn"
                onClick={() => navigator.clipboard.readText().then(setCipher).catch(() => notify("无法读取剪贴板"))}
              >
                粘贴
              </button>
              <button className="btn" disabled={!cipher} onClick={() => { setCipher(""); inputRef.current?.focus(); }}>
                清空
              </button>
            </div>
            <ParamsBar method={method} params={params} onChange={(k, v) => setParams((p) => ({ ...p, [k]: v }))} />
            <div className="card__body">
              <textarea
                ref={inputRef}
                className="field"
                aria-label="输入内容"
                placeholder={mode === "find_flag" ? "粘贴长文本，自动查找 flag…" : "粘贴密文…"}
                value={cipher}
                onChange={(e) => setCipher(e.target.value)}
                spellCheck={false}
                autoComplete="off"
                autoFocus
              />
            </div>
          </section>

          <section className="card" aria-label="输出" aria-busy={loading}>
            <div className="card__header">
              <span className="card__title">{outputTitle}</span>
              <span className="card__meta">{outputMeta}</span>
              <span className="card__spacer" />
              {(mode === "caesar_brute" || mode === "xor_brute") && (
                <button
                  className={`btn${onlyFlags ? " btn--primary" : ""}`}
                  aria-pressed={onlyFlags}
                  onClick={() => setOnlyFlags((v) => !v)}
                >
                  只看 flag
                </button>
              )}
              {mode === "decode" && (
                <>
                  <button className="btn" disabled={!result} onClick={() => chainInput(result)} title="多层编码时继续解">
                    ↵ 作为输入
                  </button>
                  <button className="btn btn--primary" disabled={!result} onClick={() => copy(result)}>
                    复制
                  </button>
                </>
              )}
            </div>
            <FlagBanner flags={flags} onCopy={copy} />
            <div className="card__body">
              {loading && <div className="loading-bar" aria-hidden />}
              {body}
            </div>
          </section>
        </div>
      </main>

      {toast && <div className="toast" role="status">{toast}</div>}
    </div>
  );
}
