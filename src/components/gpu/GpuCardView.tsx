import { useTranslation } from "react-i18next";
import type { GpuCard } from "../../api/types";
import { fmtNumber } from "../servers/format";
import { cardAlerts, reasons } from "./alerts";
import { StateMark } from "../common/StateMark";

/** Полоса заполнения 0…100 %. */
function Bar({ pct }: { pct: number }) {
  return (
    <span className="gpu-bar" aria-hidden="true">
      <span style={{ width: `${Math.max(0, Math.min(100, pct))}%` }} />
    </span>
  );
}

/**
 * Карточка карты (спека I1d §5): номер и модель, утилизация, активность SM (запасная — с пометкой), память, температура,
 * мощность, подпись экспортёра (если номер карты повторяется у хоста), значки проблем с расшифровкой, нагрузка Kubernetes и экземпляры MIG. Поле без серии — «нет данных».
 */
export function GpuCardView({
  card,
  selected,
  showExporter = false,
  onSelect,
}: {
  card: GpuCard;
  selected: boolean;
  /** Номер карты повторяется у хоста — подписать экспортёр. */
  showExporter?: boolean;
  onSelect: () => void;
}) {
  const { t } = useTranslation();
  const v = card.values;
  const none = <span className="muted">{t("gpu.card.noData")}</span>;
  const pct = (x: number | undefined) => (x == null ? none : t("gpu.units.percent", { value: fmtNumber(x) }));
  const alerts = cardAlerts(card);
  const allReasons = v.throttle != null ? reasons(v.throttle) : [];
  return (
    <section className={selected ? "gpu-card selected" : "gpu-card"} aria-label={t("gpu.card.title", { index: card.index })}>
      <button type="button" className="link gpu-card-title" aria-pressed={selected} onClick={onSelect} title={card.uuid}>
        <strong>{t("gpu.card.title", { index: card.index })}</strong> <span className="muted">{card.model ?? ""}</span>
      </button>
      {showExporter && card.exporter && <p className="muted">{t("gpu.card.exporter", { exporter: card.exporter })}</p>}
      <dl className="facts">
        <dt>{t("gpu.card.util")}</dt>
        <dd>
          {v.util != null && <Bar pct={v.util} />} {pct(v.util)}
        </dd>
        <dt>{t("gpu.card.sm")}</dt>
        <dd>
          {v.smActive != null && <Bar pct={v.smActive} />} {pct(v.smActive)}
          {card.smSource === "gr" && <span className="muted" title={t("gpu.card.smGrHint")}> {t("gpu.card.smGr")}</span>}
        </dd>
        <dt>{t("gpu.card.memory")}</dt>
        <dd>
          {v.memUsed != null && v.memTotal ? <Bar pct={(v.memUsed / v.memTotal) * 100} /> : null}{" "}
          {v.memUsed != null && v.memTotal != null
            ? t("gpu.card.memoryValue", { used: fmtNumber(v.memUsed / 1024, 1), total: fmtNumber(v.memTotal / 1024, 1) })
            : none}
        </dd>
        <dt>{t("gpu.card.temp")}</dt>
        <dd>{v.temp == null ? none : t("gpu.units.celsius", { value: fmtNumber(v.temp) })}</dd>
        <dt>{t("gpu.card.power")}</dt>
        <dd>{v.power == null ? none : t("gpu.units.watt", { value: fmtNumber(v.power) })}</dd>
      </dl>
      {alerts.length > 0 && (
        <ul className="kv gpu-alerts">
          {alerts.map((a) => (
            <li key={a.kind} className={a.severity === "danger" ? "srv-error" : "warn"} title={t(`gpu.alerts.${a.kind}Hint`, a.params)}>
              <StateMark state="warn" /> {t(`gpu.alerts.${a.kind}`, a.params)}
            </li>
          ))}
        </ul>
      )}
      {alerts.length === 0 && allReasons.length > 0 && <p className="muted">{t("gpu.card.clockReasons", { reasons: allReasons.join(", ") })}</p>}
      {card.workload && <p className="muted">{t("gpu.card.workload", { namespace: card.workload.namespace, pod: card.workload.pod })}</p>}
      {card.mig && card.mig.length > 0 && (
        <p className="muted">{t("gpu.card.mig", { instances: card.mig.map((m) => `${m.profile} #${m.instance}`).join(", ") })}</p>
      )}
    </section>
  );
}
