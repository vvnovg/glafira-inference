import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toAppError } from "../../api/client";
import type { AppError, HistoryAuth, HistoryCheck, HistorySource, HistoryTarget, InstallationProfile } from "../../api/types";
import { tr } from "../../i18n/text";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";
import { type CaMode, type ProxyMode, stepMark, withCaMode, withProxyMode } from "./profile";

const CA_MODES: CaMode[] = ["system", "pem", "insecure"];
const PROXY_MODES: ProxyMode[] = ["system", "none", "url"];
const AUTH_MODES: HistoryAuth["mode"][] = ["none", "bearer", "basic"];

/** Значение списка «Серии в истории»: свой селектор и «не сопоставлять». */
const CUSTOM = "\u0000custom";
const NONE = "";

/** Секрет источника в форме: введённое значение и «удалить сохранённый». */
export interface SecretInput {
  value: string;
  clear: boolean;
}

export function newHistory(): HistorySource {
  return { url: "", auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } };
}

/** Вид входа; имя пользователя сохраняется при переключении туда и обратно. */
export function withAuthMode(auth: HistoryAuth, mode: HistoryAuth["mode"], lastUser = ""): HistoryAuth {
  if (mode === "basic") return { mode, username: auth.mode === "basic" ? auth.username : lastUser };
  return { mode };
}

/** Подпись цели: `job · instance (vLLM)`. */
const targetLabel = (t: HistoryTarget) => `${t.job} · ${t.instance} (${t.kind === "sglang" ? "SGLang" : "vLLM"})`;

/** Селектор для показа: `{job="vllm", instance="h:8000"}` — `vllm · h:8000`, остальные — как есть. */
export function selectorLabel(selector: string): string {
  const m = /^\{\s*job="([^"\\]*)",\s*instance="([^"\\]*)"\s*\}$/.exec(selector.trim());
  return m ? `${m[1]} · ${m[2]}` : selector;
}

/** Селектор цели — как его строит Rust (`Target::selector`). */
export const targetSelector = (t: HistoryTarget) =>
  `{job="${t.job.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}", instance="${t.instance.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"}`;

interface Props {
  profile: InstallationProfile;
  idPrefix: string;
  /** У источника сохранён секрет: пустое поле его оставляет. */
  savedSecret: boolean;
  secret: SecretInput;
  insecureForbidden: boolean;
  busy: boolean;
  onHistory: (history: HistorySource | undefined) => void;
  onSecret: (patch: Partial<SecretInput>) => void;
  onSelector: (serverId: string, selector: string | undefined) => void;
  /** Проверка источника профиля формы с секретом формы. */
  check: () => Promise<HistoryCheck>;
}

/**
 * Раздел формы «История метрик» (спека I1c §5): источник, его проверка и сопоставление серверов с целями.
 * Автоподбор проверки ставит селектор только серверам без него — выбор человека не перезаписывается.
 */
export function HistoryFields({ profile, idPrefix, savedSecret, secret, insecureForbidden, busy, onHistory, onSecret, onSelector, check }: Props) {
  const { t } = useTranslation();
  const id = (field: string) => `${idPrefix}-${field}`;
  const h = profile.history;
  const [result, setResult] = useState<HistoryCheck | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  // Серверы с автоподобранным селектором и адрес, под который он подобран.
  const [auto, setAuto] = useState<Map<string, string>>(new Map());
  const [custom, setCustom] = useState<Set<string>>(new Set());
  const [lastPem, setLastPem] = useState("");
  const [lastProxy, setLastProxy] = useState("");
  const [lastUser, setLastUser] = useState("");

  // Адрес сервера сменился — подобранный под старый адрес селектор сбрасывается (выбор человека не трогаем).
  useEffect(() => {
    const stale = profile.servers.filter((s) => auto.has(s.id) && auto.get(s.id) !== s.baseUrl);
    if (stale.length === 0) return;
    for (const s of stale) onSelector(s.id, undefined);
    setAuto((a) => new Map([...a].filter(([id]) => !stale.some((s) => s.id === id))));
  }, [profile.servers, auto, onSelector]);

  const onCheck = async () => {
    setChecking(true);
    setError(null);
    setResult(null);
    try {
      const r = await check();
      setResult(r);
      const picked = new Map<string, string>();
      for (const s of r.suggestions) {
        const server = profile.servers.find((x) => x.id === s.serverId);
        if (s.selector && server && !server.historySelector) {
          onSelector(s.serverId, s.selector);
          picked.set(s.serverId, server.baseUrl);
        }
      }
      setAuto((a) => new Map([...a, ...picked]));
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setChecking(false);
    }
  };

  // Форма открыта с сохранённым источником — проверка запускается сама: без неё нет списков целей.
  const autoChecked = useRef(false);
  useEffect(() => {
    if (autoChecked.current) return;
    autoChecked.current = true;
    if (h?.url.trim()) void onCheck();
    // Только при открытии формы: дальше проверку запускает человек.
  }, []);

  if (!h) {
    return (
      <section className="history-source" aria-label={t("installations.history.title")}>
        <h3 className="form-section">{t("installations.history.title")}</h3>
        <p className="muted">{t("installations.history.none")}</p>
        <div className="toolbar">
          <button type="button" disabled={busy} onClick={() => onHistory(newHistory())}>
            {t("installations.history.add")}
          </button>
        </div>
      </section>
    );
  }

  const set = (patch: Partial<HistorySource>) => onHistory({ ...h, ...patch });
  const targets = result?.targets ?? [];
  const choose = (serverId: string, value: string) => {
    setAuto((a) => new Map([...a].filter(([x]) => x !== serverId)));
    setCustom((c) => {
      const next = new Set(c);
      if (value === CUSTOM) next.add(serverId);
      else next.delete(serverId);
      return next;
    });
    if (value !== CUSTOM) onSelector(serverId, value === NONE ? undefined : value);
  };

  return (
    <section className="history-source" aria-label={t("installations.history.title")}>
      <h3 className="form-section">{t("installations.history.title")}</h3>
      <div className="form">
        <span className="field-label">
          <label htmlFor={id("url")}>{t("installations.history.url")}</label>
          <Hint k="hints.installations.historyUrl" label={t("installations.history.url")} />
        </span>
        <input id={id("url")} placeholder="http://prometheus.monitoring:9090" value={h.url} onChange={(e) => set({ url: e.target.value })} />

        <label htmlFor={id("auth")}>{t("installations.history.auth")}</label>
        <select
          id={id("auth")}
          value={h.auth.mode}
          onChange={(e) => {
            if (h.auth.mode === "basic") setLastUser(h.auth.username);
            set({ auth: withAuthMode(h.auth, e.target.value as HistoryAuth["mode"], lastUser) });
          }}
        >
          {AUTH_MODES.map((m) => (
            <option key={m} value={m}>
              {t(`installations.history.authMode.${m}`)}
            </option>
          ))}
        </select>
        {h.auth.mode === "basic" && (
          <>
            <label htmlFor={id("user")}>{t("installations.history.username")}</label>
            <input id={id("user")} autoComplete="off" value={h.auth.username} onChange={(e) => set({ auth: { mode: "basic", username: e.target.value } })} />
          </>
        )}
        {h.auth.mode !== "none" && (
          <>
            <label htmlFor={id("secret")}>{t(h.auth.mode === "bearer" ? "installations.history.token" : "installations.history.password")}</label>
            <input
              id={id("secret")}
              type="password"
              autoComplete="off"
              placeholder={savedSecret ? t("installations.form.apiKeyKeep") : ""}
              disabled={savedSecret && secret.clear}
              value={secret.value}
              onChange={(e) => onSecret({ value: e.target.value })}
            />
            {savedSecret && (
              <label className="inline full">
                <input type="checkbox" checked={secret.clear} onChange={(e) => onSecret({ clear: e.target.checked })} /> {t("installations.history.secretClear")}
              </label>
            )}
          </>
        )}

        <label htmlFor={id("ca")}>{t("installations.form.ca")}</label>
        <select
          id={id("ca")}
          value={h.ca.mode}
          onChange={(e) => {
            if (h.ca.mode === "pem") setLastPem(h.ca.pem);
            set({ ca: withCaMode(h.ca, e.target.value as CaMode, lastPem) });
          }}
        >
          {CA_MODES.map((m) => (
            <option key={m} value={m} disabled={m === "insecure" && insecureForbidden && h.ca.mode !== "insecure"}>
              {t(`installations.form.caMode.${m}`)}
            </option>
          ))}
        </select>
        {h.ca.mode === "pem" && (
          <textarea
            className="full"
            aria-label={t("installations.form.caPem")}
            rows={4}
            placeholder="-----BEGIN CERTIFICATE-----"
            value={h.ca.pem}
            onChange={(e) => set({ ca: { mode: "pem", pem: e.target.value } })}
          />
        )}
        {h.ca.mode === "insecure" && (
          <p className="warn full" role="alert">
            {insecureForbidden ? t("installations.form.insecureForbidden") : t("installations.form.insecureWarning")}
          </p>
        )}

        <label htmlFor={id("proxy")}>{t("installations.form.proxy")}</label>
        <select
          id={id("proxy")}
          value={h.proxy.mode}
          onChange={(e) => {
            if (h.proxy.mode === "url") setLastProxy(h.proxy.url);
            set({ proxy: withProxyMode(h.proxy, e.target.value as ProxyMode, lastProxy) });
          }}
        >
          {PROXY_MODES.map((m) => (
            <option key={m} value={m}>
              {t(`installations.form.proxyMode.${m}`)}
            </option>
          ))}
        </select>
        {h.proxy.mode === "url" && (
          <input
            className="full"
            aria-label={t("installations.form.proxyUrl")}
            placeholder="http://proxy.example.com:3128"
            value={h.proxy.url}
            onChange={(e) => set({ proxy: { mode: "url", url: e.target.value } })}
          />
        )}
      </div>
      <div className="toolbar">
        <button type="button" disabled={busy || checking} onClick={() => void onCheck()}>
          {t("installations.history.check")}
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            setResult(null);
            setError(null);
            onHistory(undefined);
          }}
        >
          {t("installations.history.remove")}
        </button>
        {checking && <span className="muted">{t("installations.form.testing")}</span>}
      </div>
      {error && <ErrorBanner error={error} />}
      {result && (
        <ul className="steps" aria-label={t("installations.history.report")}>
          {result.report.steps.map((step, i) => (
            <li key={`${step.name}-${i}`} className={step.warning ? "warn" : undefined}>
              {stepMark(step)} <strong>{t(`installations.form.step.${step.name}`)}:</strong> <span>{tr(step.message)}</span>
              {step.error?.details && <pre className="muted">{step.error.details}</pre>}
            </li>
          ))}
          {result.report.warnings.map((w, i) => (
            <li key={`${w.key}-${i}`} className="warn">
              ⚠ {tr(w)}
            </li>
          ))}
        </ul>
      )}

      <h4>{t("installations.history.series")}</h4>
      {/* Цели — только из результата проверки: в заново открытой форме их ещё нет. */}
      {!result && <p className="muted">{t("installations.history.checkForTargets")}</p>}
      <div className="form">
        {profile.servers.map((s) => {
          const sel = s.historySelector;
          const known = targets.find((x) => targetSelector(x) === sel);
          // Сохранённая цель, которой нет среди найденных (проверки ещё не было или цель пропала), —
          // своим пунктом с подписью цели, а не сырым селектором.
          const savedTarget = sel && !known && selectorLabel(sel) !== sel ? sel : undefined;
          const value = custom.has(s.id) || (sel && !known && !savedTarget) ? CUSTOM : (sel ?? NONE);
          const name = s.name.trim() || s.baseUrl || s.id;
          return (
            <div key={s.id} className="contents">
              <span className="field-label">
                <label htmlFor={id(`sel-${s.id}`)}>{name}</label>
                {auto.has(s.id) && sel && <span className="muted"> {t("installations.history.auto")}</span>}
              </span>
              <select id={id(`sel-${s.id}`)} value={value} onChange={(e) => choose(s.id, e.target.value)}>
                <option value={NONE}>{t("installations.history.noMatch")}</option>
                {savedTarget && <option value={savedTarget}>{selectorLabel(savedTarget)}</option>}
                {targets.map((x) => (
                  <option key={targetSelector(x)} value={targetSelector(x)}>
                    {targetLabel(x)}
                  </option>
                ))}
                <option value={CUSTOM}>{t("installations.history.custom")}</option>
              </select>
              {value === CUSTOM && (
                <input
                  className="full"
                  aria-label={t("installations.history.selector", { server: name })}
                  placeholder='{job="vllm", instance="10.0.0.5:8000"}'
                  value={sel ?? ""}
                  onChange={(e) => onSelector(s.id, e.target.value || undefined)}
                />
              )}
            </div>
          );
        })}
      </div>
    </section>
  );
}
