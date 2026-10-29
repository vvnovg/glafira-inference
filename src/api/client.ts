import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AppInfo,
  ExportSummary,
  ImportPreview,
  InstallationFolder,
  InstallationInfo,
  InstallationProfile,
  LanguageSettings,
  PolicyStatus,
  SecretsInput,
  TestReport,
  TreeItemRef,
  UpdateCheck,
} from "./types";

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

export function toAppError(e: unknown): AppError {
  // Ошибка из Rust приходит целиком: key/params/details сохраняются как есть.
  if (isAppError(e)) return e;
  if (e instanceof Error) return { code: "INTERNAL", message: e.message, retryable: false };
  return { code: "INTERNAL", message: String(e), retryable: false };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}

export const api = {
  getAppInfo: () => call<AppInfo>("get_app_info"),
  getLanguageSettings: () => call<LanguageSettings>("get_language_settings"),
  setLanguage: (lang: string | null) => call<LanguageSettings>("set_language", { lang }),
  getPolicyStatus: () => call<PolicyStatus>("get_policy_status"),
  checkUpdate: () => call<UpdateCheck>("check_update"),
  installUpdate: () => call<void>("install_update"),

  listInstallations: () => call<InstallationProfile[]>("list_installations"),
  saveInstallation: (profile: InstallationProfile, secrets: SecretsInput = {}) =>
    call<InstallationProfile>("save_installation", { input: { profile, secrets } }),
  deleteInstallation: (id: string) => call<void>("delete_installation", { id }),
  /** Проверка сервера `serverId` несохранённого профиля: ключ из формы — поверх keychain. */
  testServer: (profile: InstallationProfile, serverId: string, secrets: SecretsInput = {}) =>
    call<TestReport>("test_server_connection", { input: { profile, secrets }, serverId }),
  /** `fresh` — подключиться заново, а не взять подключение из пула. */
  connectInstallation: (installationId: string, fresh = false) =>
    call<InstallationInfo>("connect_installation", { installationId, fresh }),
  disconnectInstallation: (installationId: string) => call<void>("disconnect_installation", { installationId }),

  listFolders: () => call<InstallationFolder[]>("list_folders"),
  createFolder: (name: string, parentId: string | null) =>
    call<InstallationFolder>("create_folder", { name, parentId }),
  renameFolder: (id: string, name: string) => call<InstallationFolder>("rename_folder", { id, name }),
  deleteFolder: (id: string) => call<void>("delete_folder", { id }),
  moveItems: (items: TreeItemRef[], parentId: string | null, before: TreeItemRef | null) =>
    call<void>("move_items", { items, parentId, before }),
  exportInstallations: (path: string, folderId: string | null) =>
    call<ExportSummary>("export_installations", { path, folderId }),
  previewImport: (path: string, targetFolderId: string | null, updateExisting: boolean) =>
    call<ImportPreview>("preview_import", { path, targetFolderId, updateExisting }),
  applyImport: (path: string, targetFolderId: string | null, updateExisting: boolean) =>
    call<ImportPreview>("apply_import", { path, targetFolderId, updateExisting }),
};
