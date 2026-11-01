import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toAppError } from "../../api/client";
import type { AppError, DcgmTarget, GpuAuth, GpuBinding, GpuCheck, GpuExporter, GpuSource, InstallationProfile } from "../../api/types";
import { tr } from "../../i18n/text";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";
import type { SecretInput } from "./HistoryFields";
import { selectorLabel } from "./HistoryFields";
import { type CaMode, type ProxyMode, stepMark, withCaMode, withProxyMode } from "./profile";

const CA_MODES: CaMode[] = ["system", "pem", "insecure"];
const PROXY_MODES: ProxyMode[] = ["system", "none", "url"];
type Mode = "none" | GpuSource["mode"];
const MODES: Mode[] = ["none", "direct", "prometheus"];
/** Сколько экспортёров может быть у компонента (как `MAX_EXPORTERS` в Rust). */
export const MAX_EXPORTERS = 16;
/** Сколько найденных хостов показывает выбор хостов селектора. */
const MAX_HOSTS = 20;

const CUSTOM = "\u0000custom";
const NONE = "";

export function newDirectGpu(): GpuSource {
  return { mode: "direct", exporters: [{ url: "" }], auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } };
}

/** Компонент в режиме `mode`; адреса экспортёров сохраняются при переключении туда и обратно. */
export function withGpuMode(gpu: GpuSource | undefined, mode: Mode, lastDirect?: GpuSource): GpuSource | undefined {
  if (mode === "none") return undefined;
  if (mode === "prometheus") return gpu?.mode === "prometheus" ? gpu : { mode: "prometheus" };
  if (gpu?.mode === "direct") return gpu;
  return lastDirect?.mode === "direct" ? lastDirect : newDirectGpu();
}

/** Селектор цели экспортёра — как его строит Rust (`DcgmTarget::selector`). */
export const dcgmSelector = (t: DcgmTarget) =>
  `{job="${t.job.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}", instance="${t.instance.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"}`;

const reEscape = (v: string) => v.replace(/[\\.+*?()|[\]{}^$]/g, "\\$&");

/** Хосты селектора вида `{label=~"a|b"}` (как его строит автоподбор); иной селектор — `null`. */
export function selectorHosts(selector: string | undefined): { label: string; hosts: string[] } | null {
  const m = /^\{\s*(hostname|Hostname)\s*=~\s*"((?:[^"\\]|\\.)*)"\s*\}$/.exec(selector?.trim() ?? "");
  if (!m) return null;
  const value = m[2].replace(/\\\\/g, "\\");
  return { label: m[1], hosts: value ? value.split("|").map((h) => h.replace(/\\(.)/g, "$1")) : [] };
}

/** Селектор хостов: `{hostname=~"a|b"}`; без хостов — `undefined` (все карты источника). */
export function hostsSelector(label: string, hosts: string[]): string | undefined {
  if (hosts.length === 0) return undefined;
  const value = [...hosts].sort().map(reEscape).join("|").replace(/\\/g, "\\\\");
  return `{${label}=~"${value}"}`;
}

interface Props {
  profile: InstallationProfile;
  idPrefix: string;
  /** У экспортёров сохранён пароль: пустое поле его оставляет. */
  savedSecret: boolean;
  secret: SecretInput;
  insecureForbidden: boolean;
  busy: boolean;
  result: GpuCheck | null;
  onResult: (r: GpuCheck | null) => void;
  onGpu: (gpu: GpuSource | undefined) => void;
  onSecret: (patch: Partial<SecretInput>) => void;
  onBinding: (serverId: string, binding: GpuBinding | undefined) => void;
  check: () => Promise<GpuCheck>;
}

/**
 * Раздел формы «GPU (DCGM)» (спека I1d §5): режим, экспортёры с входом, CA и прокси или селектор карт источника
 * истории, проверка и автоподбор. Автоподбор ставит привязку серверам без неё, селектор истории — экспортёрам без него и
 * селектор карт — пустому: выбор человека не перезаписывается. Подобранная под старый адрес привязка сбрасывается.
 */
export function GpuFields({ profile, idPrefix, savedSecret, secret, insecureForbidden, busy, result, onResult, onGpu, onSecret, onBinding, check }: Props) {
  const { t } = useTranslation();
  const id = (field: string) => `${idPrefix}-${field}`;
  const g = profile.gpu;
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  // Серверы с подобранной привязкой и адрес, под который она подобрана; экспортёры с подобранным селектором.
  const [auto, setAuto] = useState<Map<string, string>>(new Map());
  const [autoExporters, setAutoExporters] = useState<Set<string>>(new Set());
  const [autoSelector, setAutoSelector] = useState(false);
  const [custom, setCustom] = useState<Set<number>>(new Set());
  const [lastDirect, setLastDirect] = useState<GpuSource | undefined>(undefined);
  const [lastPem, setLastPem] = useState("");
  const [lastProxy, setLastProxy] = useState("");

  useEffect(() => {
    const stale = profile.servers.filter((s) => auto.has(s.id) && auto.get(s.id) !== s.baseUrl);
    if (stale.length === 0) return;
    for (const s of stale) onBinding(s.id, undefined);
    setAuto((a) => new Map([...a].filter(([sid]) => !stale.some((s) => s.id === sid))));
  }, [profile.servers, auto, onBinding]);

  const onCheck = async () => {
    setChecking(true);
    setError(null);
    onResult(null);
    try {
      const r = await check();
      onResult(r);
      const picked = new Map<string, string>();
      for (const b of r.bindings) {
        const server = profile.servers.find((x) => x.id === b.serverId);
        if (b.binding && server && !server.gpus) {
          onBinding(b.serverId, b.binding);
          picked.set(b.serverId, server.baseUrl);
        }
      }
      setAuto((a) => new Map([...a, ...picked]));
      if (profile.gpu?.mode === "prometheus" && !profile.gpu.selector && r.selector) {
        onGpu({ ...profile.gpu, selector: r.selector });
        setAutoSelector(true);
      }
      if (profile.gpu?.mode === "direct") {
        const urls = new Set<string>();
        const exporters = profile.gpu.exporters.map((e) => {
          const s = r.exporters.find((x) => x.url === e.url);
          if (!e.historySelector && s?.historySelector) {
            urls.add(e.url);
            return { ...e, historySelector: s.historySelector };
          }
          return e;
        });
        if (urls.size > 0) {
          onGpu({ ...profile.gpu, exporters });
          setAutoExporters((a) => new Set([...a, ...urls]));
        }
      }
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setChecking(false);
    }
  };

  // Форма открыта с сохранённым компонентом — проверка запускается сама: без неё нет списков хостов и целей.
  const autoChecked = useRef(false);
  useEffect(() => {
    if (autoChecked.current) return;
    autoChecked.current = true;
    if (g) void onCheck();
    // Только при открытии формы: дальше проверку запускает человек.
  }, []);

  const setMode = (mode: Mode) => {
    if (g?.mode === "direct") setLastDirect(g);
    if (mode === "none") onSecret({ value: "", clear: false });
    onResult(null);
    onGpu(withGpuMode(g, mode, lastDirect));
  };

  return (
    <section className="history-source" aria-label={t("installations.gpu.title")}>
      <h3 className="form-section">{t("installations.gpu.title")}</h3>
      <div className="form">
        <span className="field-label">
          <label htmlFor={id("mode")}>{t("installations.gpu.mode")}</label>
          <Hint k="hints.installations.gpuMode" label={t("installations.gpu.mode")} />
        </span>
        <select id={id("mode")} value={g?.mode ?? "none"} onChange={(e) => setMode(e.target.value as Mode)}>
          {MODES.map((m) => (
            <option key={m} value={m} disabled={m === "prometheus" && !profile.history && g?.mode !== "prometheus"}>
              {t(`installations.gpu.modes.${m}`)}
            </option>
          ))}
        </select>
        {!profile.history && g?.mode !== "prometheus" && <p className="muted full">{t("installations.gpu.prometheusNeedsHistory")}</p>}
        {!profile.history && g?.mode === "prometheus" && (
          <p className="warn full" role="alert">
            {t("installations.gpu.historyRemoved")}
          </p>
        )}
      </div>
      {g?.mode === "direct" && (
        <DirectFields
          gpu={g}
          id={id}
          withHistory={!!profile.history}
          result={result}
          savedSecret={savedSecret}
          secret={secret}
          insecureForbidden={insecureForbidden}
          busy={busy}
          autoExporters={autoExporters}
          custom={custom}
          lastPem={lastPem}
          lastProxy={lastProxy}
          onGpu={onGpu}
          onSecret={onSecret}
          onCustom={(index, on) =>
            setCustom((c) => {
              const next = new Set(c);
              if (on) next.add(index);
              else next.delete(index);
              return next;
            })
          }
          onManual={(url) => setAutoExporters((a) => new Set([...a].filter((x) => x !== url)))}
          onLastPem={setLastPem}
          onLastProxy={setLastProxy}
        />
      )}
      {g?.mode === "prometheus" && (
        <div className="form">
          <span className="field-label">
            <label htmlFor={id("selector")}>{t("installations.gpu.selector")}</label>
            <Hint k="hints.installations.gpuSelector" label={t("installations.gpu.selector")} />
          </span>
          <input
            id={id("selector")}
            placeholder='{hostname=~"gpu-node-1|gpu-node-2"}'
            value={g.selector ?? ""}
            onChange={(e) => {
              setAutoSelector(false);
              onGpu({ ...g, selector: e.target.value || undefined });
            }}
          />
          {autoSelector && g.selector && <span className="muted full">{t("installations.gpu.auto")}</span>}
          {result && result.available.length > 0 && (
            <fieldset className="full" aria-label={t("installations.gpu.hosts")}>
              <legend>{t("installations.gpu.hosts")}</legend>
              {result.available.slice(0, MAX_HOSTS).map((h) => {
                const parsed = selectorHosts(g.selector);
                const checked = parsed?.hosts.includes(h) ?? false;
                return (
                  <label key={h} className="inline">
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={(e) => {
                        const label = parsed?.label ?? selectorHosts(result.selector)?.label ?? "hostname";
                        const hosts = new Set(parsed?.hosts ?? []);
                        if (e.target.checked) hosts.add(h);
                        else hosts.delete(h);
                        setAutoSelector(false);
                        onGpu({ ...g, selector: hostsSelector(label, [...hosts]) });
                      }}
                    />{" "}
                    {h}
                  </label>
                );
              })}
              {result.available.length > MAX_HOSTS && <span className="muted">{t("installations.gpu.moreHosts", { count: result.available.length - MAX_HOSTS })}</span>}
            </fieldset>
          )}
        </div>
      )}
      {g && (
        <div className="toolbar">
          <button type="button" disabled={busy || checking} onClick={() => void onCheck()}>
            {t("installations.gpu.check")}
          </button>
          <Hint k="hints.installations.gpuCheck" label={t("installations.gpu.check")} />
          {checking && <span className="muted">{t("installations.form.testing")}</span>}
        </div>
      )}
      {error && <ErrorBanner error={error} />}
      {result && (
        <ul className="steps" aria-label={t("installations.gpu.report")}>
          {result.report.steps.map((step, i) => (
            <li key={`${step.name}-${i}`} className={step.warning ? "warn" : undefined}>
              {stepMark(step)} <strong>{t(`installations.form.step.${step.name}`)}:</strong> <span>{tr(step.message)}</span>
              {step.error?.details && <pre className="muted">{step.error.details}</pre>}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** Поля режима `direct`: экспортёры, вход, CA, прокси и серии экспортёров в истории. */
function DirectFields({
  gpu,
  id,
  withHistory,
  result,
  savedSecret,
  secret,
  insecureForbidden,
  busy,
  autoExporters,
  custom,
  lastPem,
  lastProxy,
  onGpu,
  onSecret,
  onCustom,
  onManual,
  onLastPem,
  onLastProxy,
}: {
  gpu: Extract<GpuSource, { mode: "direct" }>;
  id: (field: string) => string;
  withHistory: boolean;
  result: GpuCheck | null;
  savedSecret: boolean;
  secret: SecretInput;
  insecureForbidden: boolean;
  busy: boolean;
  autoExporters: Set<string>;
  custom: Set<number>;
  lastPem: string;
  lastProxy: string;
  onGpu: (gpu: GpuSource) => void;
  onSecret: (patch: Partial<SecretInput>) => void;
  onCustom: (index: number, on: boolean) => void;
  onManual: (url: string) => void;
  onLastPem: (pem: string) => void;
  onLastProxy: (url: string) => void;
}) {
  const { t } = useTranslation();
  const set = (patch: Partial<typeof gpu>) => onGpu({ ...gpu, ...patch });
  const setExporter = (index: number, patch: Partial<GpuExporter>) =>
    set({ exporters: gpu.exporters.map((e, i) => (i === index ? { ...e, ...patch } : e)) });
  const auth = gpu.auth;
  return (
    <>
      <h4>{t("installations.gpu.exporters")}</h4>
      <div className="form">
        {gpu.exporters.map((e, i) => {
          const targets = result?.exporters.find((x) => x.url === e.url)?.targets ?? [];
          const sel = e.historySelector;
          const known = targets.some((x) => dcgmSelector(x) === sel);
          const saved = sel && !known && selectorLabel(sel) !== sel ? sel : undefined;
          const value = custom.has(i) || (sel && !known && !saved) ? CUSTOM : (sel ?? NONE);
          return (
            <div key={i} className="contents">
              <label htmlFor={id(`exp-${i}`)}>{t("installations.gpu.exporter", { n: i + 1 })}</label>
              <span className="path-field">
                <input id={id(`exp-${i}`)} placeholder="http://gpu-node-1:9400/metrics" value={e.url} onChange={(ev) => setExporter(i, { url: ev.target.value })} />
                {gpu.exporters.length > 1 && (
                  <button type="button" disabled={busy} onClick={() => set({ exporters: gpu.exporters.filter((_, j) => j !== i) })}>
                    {t("installations.gpu.removeExporter")}
                  </button>
                )}
              </span>
              {withHistory && (
                <>
                  <span className="field-label">
                    <label htmlFor={id(`exp-sel-${i}`)}>{t("installations.gpu.exporterSeries")}</label>
                    {autoExporters.has(e.url) && sel && <span className="muted"> {t("installations.gpu.auto")}</span>}
                  </span>
                  <select
                    id={id(`exp-sel-${i}`)}
                    value={value}
                    onChange={(ev) => {
                      onManual(e.url);
                      onCustom(i, ev.target.value === CUSTOM);
                      if (ev.target.value !== CUSTOM) setExporter(i, { historySelector: ev.target.value || undefined });
                    }}
                  >
                    <option value={NONE}>{t("installations.history.noMatch")}</option>
                    {saved && <option value={saved}>{selectorLabel(saved)}</option>}
                    {targets.map((x) => (
                      <option key={dcgmSelector(x)} value={dcgmSelector(x)}>
                        {`${x.job} · ${x.instance}`}
                      </option>
                    ))}
                    <option value={CUSTOM}>{t("installations.history.custom")}</option>
                  </select>
                  {value === CUSTOM && (
                    <input
                      className="full"
                      aria-label={t("installations.gpu.exporterSelector", { n: i + 1 })}
                      placeholder='{job="dcgm", instance="10.0.0.5:9400"}'
                      value={sel ?? ""}
                      onChange={(ev) => setExporter(i, { historySelector: ev.target.value || undefined })}
                    />
                  )}
                </>
              )}
            </div>
          );
        })}
      </div>
      <div className="toolbar">
        <button type="button" disabled={busy || gpu.exporters.length >= MAX_EXPORTERS} onClick={() => set({ exporters: [...gpu.exporters, { url: "" }] })}>
          {t("installations.gpu.addExporter")}
        </button>
      </div>
      <div className="form">
        <label htmlFor={id("auth")}>{t("installations.gpu.auth")}</label>
        <select
          id={id("auth")}
          value={auth.mode}
          onChange={(e) => set({ auth: e.target.value === "basic" ? ({ mode: "basic", username: auth.mode === "basic" ? auth.username : "" } as GpuAuth) : { mode: "none" } })}
        >
          <option value="none">{t("installations.history.authMode.none")}</option>
          <option value="basic">{t("installations.history.authMode.basic")}</option>
        </select>
        {auth.mode === "basic" && (
          <>
            <label htmlFor={id("user")}>{t("installations.history.username")}</label>
            <input id={id("user")} autoComplete="off" value={auth.username} onChange={(e) => set({ auth: { mode: "basic", username: e.target.value } })} />
            <label htmlFor={id("secret")}>{t("installations.history.password")}</label>
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
                <input type="checkbox" checked={secret.clear} onChange={(e) => onSecret({ clear: e.target.checked })} /> {t("installations.gpu.secretClear")}
              </label>
            )}
          </>
        )}
        <label htmlFor={id("ca")}>{t("installations.form.ca")}</label>
        <select
          id={id("ca")}
          value={gpu.ca.mode}
          onChange={(e) => {
            if (gpu.ca.mode === "pem") onLastPem(gpu.ca.pem);
            set({ ca: withCaMode(gpu.ca, e.target.value as CaMode, lastPem) });
          }}
        >
          {CA_MODES.map((m) => (
            <option key={m} value={m} disabled={m === "insecure" && insecureForbidden && gpu.ca.mode !== "insecure"}>
              {t(`installations.form.caMode.${m}`)}
            </option>
          ))}
        </select>
        {gpu.ca.mode === "pem" && (
          <textarea
            className="full"
            aria-label={t("installations.form.caPem")}
            rows={4}
            placeholder="-----BEGIN CERTIFICATE-----"
            value={gpu.ca.pem}
            onChange={(e) => set({ ca: { mode: "pem", pem: e.target.value } })}
          />
        )}
        {gpu.ca.mode === "insecure" && (
          <p className="warn full" role="alert">
            {insecureForbidden ? t("installations.form.insecureForbidden") : t("installations.form.insecureWarning")}
          </p>
        )}
        <label htmlFor={id("proxy")}>{t("installations.form.proxy")}</label>
        <select
          id={id("proxy")}
          value={gpu.proxy.mode}
          onChange={(e) => {
            if (gpu.proxy.mode === "url") onLastProxy(gpu.proxy.url);
            set({ proxy: withProxyMode(gpu.proxy, e.target.value as ProxyMode, lastProxy) });
          }}
        >
          {PROXY_MODES.map((m) => (
            <option key={m} value={m}>
              {t(`installations.form.proxyMode.${m}`)}
            </option>
          ))}
        </select>
        {gpu.proxy.mode === "url" && (
          <input
            className="full"
            aria-label={t("installations.form.proxyUrl")}
            placeholder="http://proxy.example.com:3128"
            value={gpu.proxy.url}
            onChange={(e) => set({ proxy: { mode: "url", url: e.target.value } })}
          />
        )}
      </div>
    </>
  );
}

/**
 * Поле «GPU» сервера (спека I1d §5): хост из найденных проверкой и карты — все или выбранные. Сохранённая привязка к
 * хосту, которого нет среди найденных, — своим пунктом с пометкой «не найден».
 */
export function GpuBindingField({
  binding,
  hosts,
  checked,
  idPrefix,
  onChange,
}: {
  binding: GpuBinding | undefined;
  hosts: { host: string; cards: number[] }[];
  /** Проверка GPU была: без неё списка хостов нет. */
  checked: boolean;
  idPrefix: string;
  onChange: (b: GpuBinding | undefined) => void;
}) {
  const { t } = useTranslation();
  const known = hosts.find((h) => binding && h.host.toLowerCase() === binding.host.toLowerCase());
  const cards = known?.cards ?? binding?.indices ?? [];
  return (
    <>
      <span className="field-label">
        <label htmlFor={`${idPrefix}-gpu`}>{t("installations.gpu.binding")}</label>
        <Hint k="hints.installations.gpuBinding" label={t("installations.gpu.binding")} />
      </span>
      <select
        id={`${idPrefix}-gpu`}
        value={binding?.host ?? ""}
        onChange={(e) => onChange(e.target.value ? { host: e.target.value } : undefined)}
      >
        <option value="">{t("installations.gpu.noBinding")}</option>
        {binding && !known && <option value={binding.host}>{t("installations.gpu.hostMissing", { host: binding.host })}</option>}
        {hosts.map((h) => (
          <option key={h.host} value={h.host}>
            {h.host}
          </option>
        ))}
      </select>
      {!checked && <p className="muted full">{t("installations.gpu.checkForHosts")}</p>}
      {binding && cards.length > 0 && (
        <fieldset className="full" aria-label={t("installations.gpu.cards")}>
          <label className="inline">
            <input type="checkbox" checked={!binding.indices} onChange={(e) => onChange({ host: binding.host, indices: e.target.checked ? undefined : [...cards] })} />{" "}
            {t("installations.gpu.allCards")}
          </label>
          {binding.indices &&
            cards.map((i) => (
              <label key={i} className="inline">
                <input
                  type="checkbox"
                  checked={binding.indices?.includes(i) ?? false}
                  onChange={(e) => {
                    const set = new Set(binding.indices ?? []);
                    if (e.target.checked) set.add(i);
                    else set.delete(i);
                    onChange({ host: binding.host, indices: [...set].sort((a, b) => a - b) });
                  }}
                />{" "}
                {t("gpu.card.title", { index: i })}
              </label>
            ))}
        </fieldset>
      )}
    </>
  );
}
