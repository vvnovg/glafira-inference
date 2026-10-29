import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api, toAppError } from "../../api/client";
import type { AppError, Environment, InstallationProfile, SecretsInput, ServerConfig, ServerKind, TestReport } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";
import { type CaMode, isPlainHttp, newServer, type ProxyMode, withCaMode, withProxyMode } from "./profile";
import { folderPaths } from "./tree/folderTree";

const ENVIRONMENTS: Environment[] = ["local", "dev", "test", "stage", "prod"];
const KINDS: ServerKind[] = ["auto", "vllm", "sglang", "openai"];
const CA_MODES: CaMode[] = ["system", "pem", "insecure"];
const PROXY_MODES: ProxyMode[] = ["system", "none", "url"];

/** Ключ API сервера в форме: введённое значение и «удалить сохранённый». */
interface KeyInput {
  value: string;
  clear: boolean;
}

export function InstallationForm({ initial, tabId }: { initial: InstallationProfile; tabId: string }) {
  const { t } = useTranslation();
  const [profile, setProfile] = useState(initial);
  // Ключи из формы: пустое поле — оставить сохранённый (у нового сервера его нет).
  const [keys, setKeys] = useState<Record<string, KeyInput>>({});
  // Последние введённые PEM и адрес прокси по серверам: переключение режима туда и обратно их не теряет.
  const [lastPem, setLastPem] = useState<Record<string, string>>({});
  const [lastProxy, setLastProxy] = useState<Record<string, string>>({});
  const [reports, setReports] = useState<Record<string, TestReport>>({});
  const [testing, setTesting] = useState<string | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);
  // Подтверждение удаления внутри формы: нативный window.confirm в Tauri 2 не показывается и возвращает false.
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const queryClient = useQueryClient();
  const foldersQ = useQuery({ queryKey: ["folders"], queryFn: api.listFolders });
  const policyQ = useQuery({ queryKey: ["policy"], queryFn: api.getPolicyStatus });
  const insecureForbidden = policyQ.data?.allowInsecureTls === false;
  const paths = useMemo(() => folderPaths(foldersQ.data ?? []), [foldersQ.data]);
  // Ссылка на несуществующую папку означает корень (так её показывает и дерево).
  const knownFolder = (id: string | null | undefined): string | null =>
    !id || (foldersQ.isSuccess && !paths.some((p) => p.id === id)) ? null : id;
  // Папка, с которой форма открылась: перенос — только если пользователь сменил её в списке.
  const [loadedFolderId] = useState(initial.folderId);
  const [folderTouched, setFolderTouched] = useState(false);
  const folderId = knownFolder(folderTouched ? profile.folderId : initial.folderId);
  const open = useWorkspace((s) => s.open);
  const close = useWorkspace((s) => s.close);
  const closeForInstallation = useWorkspace((s) => s.closeForInstallation);
  const isNew = initial.id === "";
  const savedServers = new Set(initial.servers.map((s) => s.id));

  const update = (patch: Partial<InstallationProfile>) => setProfile((p) => ({ ...p, ...patch }));
  const updateServer = (id: string, patch: Partial<ServerConfig>) =>
    setProfile((p) => ({ ...p, servers: p.servers.map((s) => (s.id === id ? { ...s, ...patch } : s)) }));
  const setKey = (id: string, patch: Partial<KeyInput>) =>
    setKeys((k) => ({ ...k, [id]: { value: k[id]?.value ?? "", clear: k[id]?.clear ?? false, ...patch } }));
  const current = (): InstallationProfile => ({ ...profile, folderId });
  const secrets = (): SecretsInput => {
    const apiKeys: Record<string, string> = {};
    for (const s of profile.servers) {
      const k = keys[s.id];
      if (k?.clear) apiKeys[s.id] = "";
      else if (k?.value.trim()) apiKeys[s.id] = k.value;
    }
    return { apiKeys };
  };

  async function run(fn: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setBusy(false);
    }
  }

  const onTest = (serverId: string) =>
    run(async () => {
      setTesting(serverId);
      setReports((r) => {
        const rest = { ...r };
        delete rest[serverId];
        return rest;
      });
      try {
        const report = await api.testServer(current(), serverId, secrets());
        setReports((r) => ({ ...r, [serverId]: report }));
      } finally {
        setTesting(null);
      }
    });

  const onSave = () =>
    run(async () => {
      const saved = await api.saveInstallation(current(), secrets());
      // save_installation не меняет место существующей инсталляции в дереве — перенос отдельной командой.
      try {
        if (!isNew && folderTouched && folderId !== knownFolder(loadedFolderId)) {
          await api.moveItems([{ kind: "installation", id: saved.id }], folderId, null);
        }
      } finally {
        // Настройки уже сохранены, даже если перенос не удался: список нужно обновить в любом случае.
        await queryClient.invalidateQueries({ queryKey: ["installations"] });
      }
      // Rust забыл подключение инсталляции: вкладка подключится заново с новым профилем.
      queryClient.removeQueries({ predicate: (q) => q.queryKey[1] === saved.id });
      close(tabId);
      open({ kind: "installation", installationId: saved.id });
    });

  const onDelete = () =>
    run(async () => {
      setConfirmingDelete(false);
      await api.deleteInstallation(profile.id);
      await queryClient.invalidateQueries({ queryKey: ["installations"] });
      queryClient.removeQueries({ predicate: (q) => q.queryKey[1] === profile.id });
      closeForInstallation(profile.id);
      close(tabId);
    });

  const addServer = () => update({ servers: [...profile.servers, newServer()] });
  const removeServer = (id: string) => {
    update({ servers: profile.servers.filter((s) => s.id !== id) });
    setReports((r) => {
      const rest = { ...r };
      delete rest[id];
      return rest;
    });
  };

  return (
    <div className="page">
      <div className="toolbar">
        <h2>{isNew ? t("installations.form.titleNew") : t("installations.form.titleEdit", { name: initial.name })}</h2>
      </div>
      <div className="form">
        <label htmlFor="if-name">{t("installations.form.name")}</label>
        <input id="if-name" value={profile.name} onChange={(e) => update({ name: e.target.value })} />

        <label htmlFor="if-folder">{t("installations.form.folder")}</label>
        <select
          id="if-folder"
          value={folderId ?? ""}
          onChange={(e) => {
            setFolderTouched(true);
            update({ folderId: e.target.value || null });
          }}
        >
          <option value="">{t("installations.form.folderRoot")}</option>
          {paths.map((p) => (
            <option key={p.id} value={p.id}>
              {p.path.join(" / ")}
            </option>
          ))}
        </select>

        <span className="field-label">
          <label htmlFor="if-env">{t("installations.form.environment")}</label>
          <Hint k="hints.installations.environment" label={t("installations.form.environment")} />
        </span>
        <select
          id="if-env"
          value={profile.environment}
          onChange={(e) => {
            const environment = e.target.value as Environment;
            // Новая prod-инсталляция по умолчанию только для чтения; снять флажок можно сразу.
            update({ environment, readOnly: environment === "prod" && isNew ? true : profile.readOnly });
          }}
        >
          {ENVIRONMENTS.map((env) => (
            <option key={env} value={env}>
              {env}
            </option>
          ))}
        </select>

        <span className="field-label">
          <label htmlFor="if-ro">{t("common.readOnly")}</label>
          <Hint k="hints.installations.readOnly" label={t("common.readOnly")} />
        </span>
        <span>
          <input id="if-ro" type="checkbox" checked={profile.readOnly} onChange={(e) => update({ readOnly: e.target.checked })} />
        </span>

        <label htmlFor="if-color">{t("installations.form.color")}</label>
        <input id="if-color" type="color" value={profile.color} onChange={(e) => update({ color: e.target.value })} />

        <label htmlFor="if-timeout">{t("installations.form.requestTimeout")}</label>
        <input
          id="if-timeout"
          type="number"
          min={1000}
          step={1000}
          value={profile.requestTimeoutMs}
          onChange={(e) => update({ requestTimeoutMs: Number(e.target.value) })}
        />

        <label htmlFor="if-ctimeout">{t("installations.form.connectTimeout")}</label>
        <input
          id="if-ctimeout"
          type="number"
          min={1000}
          step={1000}
          value={profile.connectTimeoutMs}
          onChange={(e) => update({ connectTimeoutMs: Number(e.target.value) })}
        />
      </div>

      <h3>{t("installations.form.servers")}</h3>
      {profile.servers.map((s, i) => (
        <ServerFields
          key={s.id}
          server={s}
          index={i}
          saved={!isNew && savedServers.has(s.id)}
          keyInput={keys[s.id] ?? { value: "", clear: false }}
          insecureForbidden={insecureForbidden}
          busy={busy}
          testing={testing === s.id}
          report={reports[s.id]}
          canRemove={profile.servers.length > 1}
          onChange={(patch) => updateServer(s.id, patch)}
          onKey={(patch) => setKey(s.id, patch)}
          onCaMode={(mode) => {
            if (s.ca.mode === "pem") {
              const pem = s.ca.pem;
              setLastPem((m) => ({ ...m, [s.id]: pem }));
            }
            updateServer(s.id, { ca: withCaMode(s.ca, mode, lastPem[s.id]) });
          }}
          onProxyMode={(mode) => {
            if (s.proxy.mode === "url") {
              const url = s.proxy.url;
              setLastProxy((m) => ({ ...m, [s.id]: url }));
            }
            updateServer(s.id, { proxy: withProxyMode(s.proxy, mode, lastProxy[s.id]) });
          }}
          onTest={() => onTest(s.id)}
          onRemove={() => removeServer(s.id)}
        />
      ))}
      <div className="toolbar">
        <button type="button" disabled={busy} onClick={addServer}>
          {t("installations.form.addServer")}
        </button>
      </div>

      <div className="toolbar">
        <button type="button" className="primary" disabled={busy} onClick={onSave}>
          {t("common.save")}
        </button>
        {!isNew &&
          (confirmingDelete ? (
            <span role="group" aria-label={t("installations.form.deleteGroup")}>
              <strong>{t("installations.form.deleteConfirm", { name: profile.name })}</strong>{" "}
              <button type="button" className="danger" disabled={busy} onClick={onDelete}>
                {t("installations.form.deleteYes")}
              </button>{" "}
              <button type="button" disabled={busy} onClick={() => setConfirmingDelete(false)}>
                {t("common.cancel")}
              </button>
            </span>
          ) : (
            <button type="button" className="danger" disabled={busy} onClick={() => setConfirmingDelete(true)}>
              {t("common.delete")}
            </button>
          ))}
        {busy && <span className="muted">{t("installations.form.busy")}</span>}
      </div>

      {error && <ErrorBanner error={error} />}
    </div>
  );
}

function ServerFields({
  server: s,
  index,
  saved,
  keyInput,
  insecureForbidden,
  busy,
  testing,
  report,
  canRemove,
  onChange,
  onKey,
  onCaMode,
  onProxyMode,
  onTest,
  onRemove,
}: {
  server: ServerConfig;
  index: number;
  /** Сервер уже сохранён: у него может быть ключ в keychain. */
  saved: boolean;
  keyInput: KeyInput;
  insecureForbidden: boolean;
  busy: boolean;
  testing: boolean;
  report?: TestReport;
  canRemove: boolean;
  onChange: (patch: Partial<ServerConfig>) => void;
  onKey: (patch: Partial<KeyInput>) => void;
  onCaMode: (mode: CaMode) => void;
  onProxyMode: (mode: ProxyMode) => void;
  onTest: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation();
  const id = (field: string) => `if-s${index}-${field}`;
  const keyGoesPlain = isPlainHttp(s.baseUrl) && !keyInput.clear && (keyInput.value.trim() !== "" || saved);
  return (
    <fieldset className="server" aria-label={t("installations.form.server", { n: index + 1 })}>
      <legend>{s.name.trim() || t("installations.form.server", { n: index + 1 })}</legend>
      <div className="form">
        <label htmlFor={id("name")}>{t("installations.form.serverName")}</label>
        <input id={id("name")} placeholder="vllm-1" value={s.name} onChange={(e) => onChange({ name: e.target.value })} />

        <span className="field-label">
          <label htmlFor={id("url")}>{t("installations.form.baseUrl")}</label>
          <Hint k="hints.installations.baseUrl" label={t("installations.form.baseUrl")} />
        </span>
        <input id={id("url")} placeholder="http://vllm.internal:8000" value={s.baseUrl} onChange={(e) => onChange({ baseUrl: e.target.value })} />

        <span className="field-label">
          <label htmlFor={id("kind")}>{t("installations.form.kind")}</label>
          <Hint k="hints.installations.kind" label={t("installations.form.kind")} />
        </span>
        <select id={id("kind")} value={s.kind} onChange={(e) => onChange({ kind: e.target.value as ServerKind })}>
          {KINDS.map((k) => (
            <option key={k} value={k}>
              {t(`installations.form.kinds.${k}`)}
            </option>
          ))}
        </select>

        <span className="field-label">
          <label htmlFor={id("key")}>{t("installations.form.apiKey")}</label>
          <Hint k="hints.installations.apiKey" label={t("installations.form.apiKey")} />
        </span>
        <input
          id={id("key")}
          type="password"
          autoComplete="off"
          placeholder={saved ? t("installations.form.apiKeyKeep") : ""}
          disabled={keyInput.clear}
          value={keyInput.value}
          onChange={(e) => onKey({ value: e.target.value })}
        />
        {saved && (
          <label className="inline full">
            <input type="checkbox" checked={keyInput.clear} onChange={(e) => onKey({ clear: e.target.checked })} />{" "}
            {t("installations.form.apiKeyClear")}
          </label>
        )}
        {keyGoesPlain && (
          <p className="warn full" role="alert">
            {t("installations.form.apiKeyOverHttp")}
          </p>
        )}

        <span className="field-label">
          <label htmlFor={id("metrics")}>{t("installations.form.metricsPath")}</label>
          <Hint k="hints.installations.metricsPath" label={t("installations.form.metricsPath")} />
        </span>
        <input id={id("metrics")} value={s.metricsPath} onChange={(e) => onChange({ metricsPath: e.target.value })} />

        <span className="field-label">
          <label htmlFor={id("ca")}>{t("installations.form.ca")}</label>
          <Hint k="hints.installations.ca" label={t("installations.form.ca")} />
        </span>
        <select id={id("ca")} value={s.ca.mode} onChange={(e) => onCaMode(e.target.value as CaMode)}>
          {CA_MODES.map((m) => (
            <option key={m} value={m} disabled={m === "insecure" && insecureForbidden && s.ca.mode !== "insecure"}>
              {t(`installations.form.caMode.${m}`)}
            </option>
          ))}
        </select>
        {s.ca.mode === "pem" && (
          <textarea
            className="full"
            aria-label={t("installations.form.caPem")}
            rows={4}
            placeholder="-----BEGIN CERTIFICATE-----"
            value={s.ca.pem}
            onChange={(e) => onChange({ ca: { mode: "pem", pem: e.target.value } })}
          />
        )}
        {s.ca.mode === "insecure" && (
          <p className="warn full" role="alert">
            {insecureForbidden ? t("installations.form.insecureForbidden") : t("installations.form.insecureWarning")}
          </p>
        )}

        <span className="field-label">
          <label htmlFor={id("proxy")}>{t("installations.form.proxy")}</label>
          <Hint k="hints.installations.proxy" label={t("installations.form.proxy")} />
        </span>
        <select id={id("proxy")} value={s.proxy.mode} onChange={(e) => onProxyMode(e.target.value as ProxyMode)}>
          {PROXY_MODES.map((m) => (
            <option key={m} value={m}>
              {t(`installations.form.proxyMode.${m}`)}
            </option>
          ))}
        </select>
        {s.proxy.mode === "url" && (
          <input
            className="full"
            aria-label={t("installations.form.proxyUrl")}
            placeholder="http://proxy.example.com:3128"
            value={s.proxy.url}
            onChange={(e) => onChange({ proxy: { mode: "url", url: e.target.value } })}
          />
        )}
      </div>
      <div className="toolbar">
        <button type="button" disabled={busy} onClick={onTest}>
          {t("installations.form.test")}
        </button>
        <Hint k="hints.installations.test" label={t("installations.form.test")} />
        {canRemove && (
          <button type="button" disabled={busy} onClick={onRemove}>
            {t("installations.form.removeServer")}
          </button>
        )}
        {testing && <span className="muted">{t("installations.form.testing")}</span>}
      </div>
      {report && (
        <ul className="steps" aria-label={t("installations.form.report")}>
          {report.steps.map((step) => (
            <li key={step.name}>
              {step.ok ? "✅" : "❌"} <strong>{t(`installations.form.step.${step.name}`)}:</strong> <span>{tr(step.message)}</span>
              {step.error?.details && <pre className="muted">{step.error.details}</pre>}
            </li>
          ))}
          {report.warnings.map((w) => (
            <li key={w.key} className="warn">
              ⚠ {tr(w)}
            </li>
          ))}
        </ul>
      )}
    </fieldset>
  );
}
