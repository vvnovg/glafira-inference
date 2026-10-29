# Перенесено из Glafira for Kubernetes

Файлы скопированы из `vvnovg/glafira-kubernetes` (локально `~/projects/glafira-openshift`) скриптом `scripts/port.sh`.
Многие из них Kubernetes-версия сама взяла из Glafira for Apache Kafka (`vvnovg/glafira-kafka`, её `PORTED.md`).
Исправление общей части нужно перенести в другие проекты; строка показывает, откуда взят файл.

| Здесь | В glafira-kubernetes | Коммит | Правки |
|---|---|---|---|
| `.gitattributes` | `.gitattributes` | `468e08a` | как есть |
| `.gitignore` | `.gitignore` | `468e08a` | адаптирован: шаблон файлов экспорта инсталляций, без стенда kind и крейтов MCP |
| `LICENSE` | `LICENSE` | `468e08a` | как есть |
| `index.html` | `index.html` | `468e08a` | адаптирован: заголовок |
| `pnpm-workspace.yaml` | `pnpm-workspace.yaml` | `468e08a` | как есть |
| `public/app-icon.svg` | `public/app-icon.svg` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `scripts/check-dist-compat.mjs` | `scripts/check-dist-compat.mjs` | `468e08a` | как есть |
| `scripts/check-dist-compat.test.mjs` | `scripts/check-dist-compat.test.mjs` | `468e08a` | как есть |
| `src-tauri/.gitignore` | `src-tauri/.gitignore` | `468e08a` | как есть |
| `src-tauri/build.rs` | `src-tauri/build.rs` | `468e08a` | как есть |
| `src-tauri/capabilities/default.json` | `src-tauri/capabilities/default.json` | `468e08a` | как есть |
| `src-tauri/icons/32x32.png` | `src-tauri/icons/32x32.png` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/128x128.png` | `src-tauri/icons/128x128.png` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/128x128@2x.png` | `src-tauri/icons/128x128@2x.png` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/icon.icns` | `src-tauri/icons/icon.icns` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/icon.ico` | `src-tauri/icons/icon.ico` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/icon.png` | `src-tauri/icons/icon.png` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src/polyfills.ts` | `src/polyfills.ts` | `468e08a` | как есть |
| `src/polyfills.test.ts` | `src/polyfills.test.ts` | `468e08a` | как есть |
| `src/styles.css` | `src/styles.css` | `468e08a` | как есть |
| `src/vite-env.d.ts` | `src/vite-env.d.ts` | `468e08a` | как есть |
| `tsconfig.json` | `tsconfig.json` | `468e08a` | как есть |
| `tsconfig.node.json` | `tsconfig.node.json` | `468e08a` | как есть |
| `vite.config.ts` | `vite.config.ts` | `468e08a` | адаптирован: порт 1440 (1420 и 1430 — у Kafka- и Kubernetes-версий) |
| `vitest.config.ts` | `vitest.config.ts` | `468e08a` | как есть |
| `NOTICE` | `NOTICE` | `468e08a` | адаптирован: имя продукта и товарные знаки |
| `TRADEMARK.md` | `TRADEMARK.md` | `468e08a` | адаптирован: имя продукта и товарные знаки |
| `SECURITY.md` | `SECURITY.md` | `468e08a` | адаптирован: имя продукта, адрес релизов, что важно для безопасности |
| `CONTRIBUTING.md` | `CONTRIBUTING.md` | `468e08a` | адаптирован: имя продукта, общий код с Kubernetes-версией |
| `deny.toml` | `deny.toml` | `468e08a` | как есть |
| `src-tauri/src/i18n.rs` | `src-tauri/src/i18n.rs` | `468e08a` | как есть |
| `src-tauri/src/error.rs` | `src-tauri/src/error.rs` | `468e08a` | как есть |
| `src-tauri/src/storage/fs.rs` | `src-tauri/src/storage/fs.rs` | `468e08a` | как есть |
| `src-tauri/src/secrets.rs` | `src-tauri/src/secrets.rs` | `468e08a` | адаптирован: ключ API сервера, ключ keychain из инсталляции, компонента и поля |
| `src-tauri/src/logging.rs` | `src-tauri/src/logging.rs` | `468e08a` | как есть |
| `src-tauri/src/sync.rs` | `src-tauri/src/sync.rs` | `468e08a` | как есть |
| `src-tauri/src/dock_name.rs` | `src-tauri/src/dock_name.rs` | `468e08a` | адаптирован: имя бинаря |
| `src-tauri/src/tls.rs` | `src-tauri/src/tls.rs` | `468e08a` | как есть |
| `src-tauri/src/policy/mod.rs` | `src-tauri/src/policy/mod.rs` | `468e08a` | адаптирован: без `allowExecAuth` (появится в I3 вместе с Kubernetes) |
| `src-tauri/src/policy/sources.rs` | `src-tauri/src/policy/sources.rs` | `468e08a` | адаптирован: без `allowExecAuth` в проверке примеров |
| `src-tauri/src/settings.rs` | `src-tauri/src/settings.rs` | `468e08a` | адаптирован: ссылка на этап I7 |
| `src-tauri/src/commands/settings.rs` | `src-tauri/src/commands/settings.rs` | `468e08a` | как есть |
| `docs/admin/policy.json` | `docs/admin/policy.json` | `468e08a` | адаптирован: идентификаторы, без `allowExecAuth` |
| `docs/admin/policy.mobileconfig` | `docs/admin/policy.mobileconfig` | `468e08a` | адаптирован: идентификаторы, без `allowExecAuth` |
| `docs/admin/policy.reg` | `docs/admin/policy.reg` | `468e08a` | адаптирован: ветка реестра, без `allowExecAuth` |
| `src-tauri/src/storage/testdata/test-cert.pem` | `src-tauri/src/storage/testdata/test-cert.pem` | `468e08a` | как есть |
| `src-tauri/src/storage/testdata/test-key.pem` | `src-tauri/src/storage/testdata/test-key.pem` | `468e08a` | как есть |
| `src-tauri/src/storage/profile.rs` | `src-tauri/src/storage/profile.rs` | `468e08a` | адаптирован: профиль инсталляции с серверами вместо профиля подключения; PEM, адреса и прокси — как есть |
| `src-tauri/src/storage/installations.rs` | `src-tauri/src/storage/connections.rs` | `468e08a` | адаптирован: инсталляции вместо подключений, неизвестные поля серверов |
| `src-tauri/src/storage/tree.rs` | `src-tauri/src/storage/tree.rs` | `468e08a` | адаптирован: инсталляции вместо подключений |
| `src-tauri/src/storage/exchange.rs` | `src-tauri/src/storage/exchange.rs` | `468e08a` | адаптирован: ключи API серверов вместо токена и ключа сертификата, без exec-плагинов и доступа LLM, предупреждения о смене адреса сервера |
| `src-tauri/src/commands/folders.rs` | `src-tauri/src/commands/folders.rs` | `468e08a` | адаптирован: удаление ключей серверов, которых нет в файле; без exec-плагинов и сессий |
| `src-tauri/src/servers/errors.rs` | `src-tauri/src/k8s/errors.rs` | `468e08a` | адаптирован: только сетевые ошибки и коды HTTP сервера инференса, ключи `errors.server.*` |
| `src-tauri/src/servers/connectivity.rs` | `src-tauri/src/k8s/connectivity.rs` | `468e08a` | адаптирован: шаги сервера инференса (здоровье, вход ключом API, тип и версия, модели), предупреждения, системный прокси |
| `src-tauri/src/commands/app.rs` | `src-tauri/src/commands/app.rs` | `468e08a` | адаптирован: только сведения «О программе», обновления — в I2 |
| `src-tauri/src/commands/installations.rs` | `src-tauri/src/commands/connections.rs` | `468e08a` | адаптирован: ключи API серверов вместо токена и ключа сертификата, проверка сервера, подключение инсталляции |
| `scripts/i18n-check.mjs` | `scripts/i18n-check.mjs` | `468e08a` | адаптирован: глав руководства пока нет |
| `scripts/i18n-check.test.mjs` | `scripts/i18n-check.test.mjs` | `468e08a` | как есть |
| `scripts/changelog.mjs` | `scripts/changelog.mjs` | `468e08a` | как есть |
| `scripts/changelog.test.mjs` | `scripts/changelog.test.mjs` | `468e08a` | как есть |
| `scripts/merge-locales.mjs` | `scripts/merge-locales.mjs` | `468e08a` | как есть |
| `scripts/merge-locales.test.mjs` | `scripts/merge-locales.test.mjs` | `468e08a` | как есть |
| `src/i18n/index.ts` | `src/i18n/index.ts` | `468e08a` | как есть |
| `src/i18n/text.ts` | `src/i18n/text.ts` | `468e08a` | как есть |
| `src/i18n/languages.ts` | `src/i18n/languages.ts` | `468e08a` | адаптирован: ссылка на этап I7 |
| `src/i18n/documentLang.test.ts` | `src/i18n/documentLang.test.ts` | `468e08a` | как есть |
| `src/i18n/locales.test.ts` | `src/i18n/locales.test.ts` | `468e08a` | как есть |
| `src/i18n/text.test.ts` | `src/i18n/text.test.ts` | `468e08a` | как есть |
| `src/lib/activate.ts` | `src/lib/activate.ts` | `468e08a` | как есть |
| `src/lib/activate.test.tsx` | `src/lib/activate.test.tsx` | `468e08a` | как есть |
| `src/lib/env.ts` | `src/lib/env.ts` | `468e08a` | как есть |
| `src/lib/focusTrap.ts` | `src/lib/focusTrap.ts` | `468e08a` | как есть |
| `src/lib/format.ts` | `src/lib/format.ts` | `468e08a` | как есть |
| `src/lib/format.test.ts` | `src/lib/format.test.ts` | `468e08a` | как есть |
| `src/lib/formatAgo.ts` | `src/lib/formatAgo.ts` | `468e08a` | как есть |
| `src/lib/links.ts` | `src/lib/links.ts` | `468e08a` | адаптирован: адреса репозиториев |
| `src/lib/plainInputs.ts` | `src/lib/plainInputs.ts` | `468e08a` | как есть |
| `src/lib/plainInputs.test.ts` | `src/lib/plainInputs.test.ts` | `468e08a` | как есть |
| `src/lib/useDebouncedValue.ts` | `src/lib/useDebouncedValue.ts` | `468e08a` | как есть |
| `src/lib/useDebouncedValue.test.ts` | `src/lib/useDebouncedValue.test.ts` | `468e08a` | как есть |
| `src/lib/useLanguageSync.ts` | `src/lib/useLanguageSync.ts` | `468e08a` | как есть |
| `src/components/common/EnvBadge.tsx` | `src/components/common/EnvBadge.tsx` | `468e08a` | как есть |
| `src/components/common/ErrorBanner.tsx` | `src/components/common/ErrorBanner.tsx` | `468e08a` | как есть |
| `src/components/common/ErrorBanner.test.tsx` | `src/components/common/ErrorBanner.test.tsx` | `468e08a` | адаптирован: ошибка сервера инференса |
| `src/components/common/Hint.tsx` | `src/components/common/Hint.tsx` | `468e08a` | как есть |
| `src/components/common/Hint.test.tsx` | `src/components/common/Hint.test.tsx` | `468e08a` | как есть |
| `src/test/render.tsx` | `src/test/render.tsx` | `468e08a` | как есть |
| `src/test/setup.ts` | `src/test/setup.ts` | `468e08a` | как есть |
