import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api } from "../../api/client";
import { ErrorBanner } from "../common/ErrorBanner";
import { InstallationForm } from "./InstallationForm";
import { newInstallation } from "./profile";

export function InstallationFormTab({
  installationId,
  folderId = null,
  tabId,
}: {
  installationId: string | null;
  /** Папка новой инсталляции; новая папка пересоздаёт форму (вкладка «новая инсталляция» одна). */
  folderId?: string | null;
  tabId: string;
}) {
  const { t } = useTranslation();
  const q = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  if (installationId === null)
    return <InstallationForm key={`new:${folderId ?? ""}`} initial={{ ...newInstallation(), folderId }} tabId={tabId} />;
  if (q.isPending) return <p className="empty">{t("common.loading")}</p>;
  if (q.error) return <ErrorBanner error={q.error} onRetry={() => q.refetch()} />;
  const profile = q.data.find((c) => c.id === installationId);
  if (!profile) return <p className="empty">{t("installations.form.deleted")}</p>;
  return <InstallationForm key={profile.id} initial={profile} tabId={tabId} />;
}
