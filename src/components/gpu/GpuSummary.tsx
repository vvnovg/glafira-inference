import { useTranslation } from "react-i18next";
import type { InstallationProfile, ServerConfig } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { fmtNumber } from "../servers/format";
import { cardAlerts } from "./alerts";
import { boundCards, hostSummary } from "./summary";
import { useGpuStatus } from "./useGpu";
import { StateMark } from "../common/StateMark";

/** Блок «GPU по хостам» «Обзора» инсталляции (спека I1d §5): строка на хост; щелчок открывает вкладку GPU. */
export function GpuHostsBlock({ profile }: { profile: InstallationProfile }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const live = useGpuStatus(profile.id, true);
  const openGpu = () => open({ kind: "gpu", installationId: profile.id });
  const pct = (v: number | null) => (v == null ? "—" : t("gpu.units.percent", { value: fmtNumber(v) }));
  return (
    <section className="card" aria-label={t("gpu.overview.title")}>
      <h4>
        <button type="button" className="link" onClick={openGpu}>
          {t("gpu.overview.title")}
        </button>
      </h4>
      {live.error && <p className="srv-error">{tr(live.error)}</p>}
      {!live.status && !live.error && <p className="muted">{t("common.loading")}</p>}
      {live.status && live.status.hosts.length === 0 && <p className="muted">{t("gpu.panel.noCards")}</p>}
      {live.status && live.status.hosts.length > 0 && (
        <table className="gpu-summary">
          <thead>
            <tr>
              <th>{t("gpu.overview.host")}</th>
              <th>{t("gpu.overview.model")}</th>
              <th>{t("gpu.overview.cards")}</th>
              <th>{t("gpu.overview.util")}</th>
              <th>{t("gpu.overview.memory")}</th>
              <th>{t("gpu.overview.temp")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {live.status.hosts.map(hostSummary).map((s) => (
              <tr key={s.host}>
                <td>
                  <button type="button" className="link" onClick={openGpu}>
                    {s.host}
                  </button>
                </td>
                <td>{s.model ?? (s.mixed ? t("gpu.overview.mixed") : "—")}</td>
                <td>{s.count}</td>
                <td>{pct(s.avgUtil)}</td>
                <td>
                  {s.memUsed == null || s.memTotal == null
                    ? "—"
                    : t("gpu.card.memoryValue", { used: fmtNumber(s.memUsed / 1024), total: fmtNumber(s.memTotal / 1024) })}
                </td>
                <td>{s.maxTemp == null ? "—" : t("gpu.units.celsius", { value: fmtNumber(s.maxTemp) })}</td>
                <td>{s.troubled > 0 && <span className="warn"><StateMark state="warn" /> {t("gpu.overview.troubled", { count: s.troubled })}</span>}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

/**
 * Блок «GPU сервера» «Обзора» сервера (спека I1d §5): строка на привязанную карту и ссылка на вкладку GPU. Без привязки —
 * одна подсказка, опроса нет.
 */
export function ServerGpuBlock({ profile, server }: { profile: InstallationProfile; server: ServerConfig }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const binding = server.gpus;
  const live = useGpuStatus(profile.id, !!binding);
  const cards = binding ? boundCards(live.status, binding) : [];
  return (
    <section className="gpu-server" aria-label={t("gpu.overview.server")}>
      <h4>{t("gpu.overview.server")}</h4>
      {!binding && <p className="muted">{t("gpu.overview.bindHint")}</p>}
      {binding && live.error && <p className="srv-error">{tr(live.error)}</p>}
      {binding && live.status && cards.length === 0 && <p className="muted">{t("gpu.overview.noBoundCards", { host: binding.host })}</p>}
      {cards.length > 0 && (
        <ul className="steps">
          {cards.map((c) => {
            const v = c.values;
            const alerts = cardAlerts(c);
            return (
              <li key={c.index}>
                <strong>{t("gpu.card.title", { index: c.index })}</strong>{" "}
                <span>
                  {[
                    v.util == null ? null : t("gpu.overview.utilValue", { value: fmtNumber(v.util) }),
                    v.memUsed == null || v.memTotal == null
                      ? null
                      : t("gpu.card.memoryValue", { used: fmtNumber(v.memUsed / 1024, 1), total: fmtNumber(v.memTotal / 1024, 1) }),
                    v.temp == null ? null : t("gpu.units.celsius", { value: fmtNumber(v.temp) }),
                  ]
                    .filter(Boolean)
                    .join(" · ") || t("gpu.card.noData")}
                </span>
                {alerts.map((a) => (
                  <span key={a.kind} className={a.severity === "danger" ? "srv-error" : "warn"} title={t(`gpu.alerts.${a.kind}Hint`, a.params)}>
                    {" "}
                    <StateMark state="warn" /> {t(`gpu.alerts.${a.kind}`, a.params)}
                  </span>
                ))}
              </li>
            );
          })}
        </ul>
      )}
      {binding && (
        <button type="button" className="link" onClick={() => open({ kind: "gpu", installationId: profile.id })}>
          {t("gpu.overview.openGpu")}
        </button>
      )}
    </section>
  );
}
