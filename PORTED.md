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
