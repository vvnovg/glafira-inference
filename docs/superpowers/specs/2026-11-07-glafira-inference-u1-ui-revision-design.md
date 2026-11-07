# Glafira for LLM Inference — этап U1 «Ревизия интерфейса под macOS» (перенос из Kubernetes-версии)

**Дата:** 2026-11-07
**Источник:** Glafira for Kubernetes, коммит `41b611c` в `main` (выпуск 0.3.0), спецификация переноса
`docs/superpowers/specs/2026-11-06-glafira-kubernetes-u1-ui-revision-design.md` там же; первоисточник — Glafira for
Apache Kafka, коммит `6a3fcfe` (выпуск 0.6.0), спецификация `docs/superpowers/specs/2026-11-06-glafira-ui-revision-design.md`.
Здесь описаны только решения переноса: что берётся как есть, что адаптируется под инсталляции инференса и почему.
Токены, типографика, элементы управления, окно, меню и тесты — по спецификации Kafka-версии (§2–§9).
**Ветка:** `feat/ui-revision-u1` от `main` (0.1.0). Версию не поднимать: выпуск — отдельное решение владельца.
**Порядок в семействе:** Kafka → Kubernetes → PostgreSQL → **LLM Inference**.

Новых функций нет. Новые строки словаря — пункты меню `menu.*` и подписи значков состояния `common.state.*` (en, ru).

## 1. Решения

| Вопрос | Решение |
|---|---|
| Общие файлы | `scripts/port.sh` получает номер коммита третьим аргументом (по умолчанию прежний `468e08a`); файлы U1 берутся с `41b611c`, строки `PORTED.md` обновляются |
| `styles.css` | Как есть из Kubernetes-версии (там — как есть из Kafka-версии). Правила экранов инсталляций остаются в `src/app.css` |
| `app.css` | Согласован с токенами: скругления `var(--radius)` (6 px) и 8 px у карточек, `.srv-ok` — `var(--ok)`, блок `.release-notes` убран (он теперь в `styles.css`), курсив подсказки поля — в `styles.css`, здесь только цвет; добавлены `.state-mark` и значки подвкладок |
| Дерево | `TreeRow` как есть (`src/components/installations/tree/TreeRow.tsx`); узлы инсталляции — §2. Вложенный `ul` даёт отступ 14 px и направляющие |
| Подсветка активной вкладки | `aria-current="page"` и класс `current` у листа: «Обзор» — вкладка `installation`, строка сервера — вкладка `server` с его `serverId`, «GPU» — вкладка `gpu` |
| Состояние сервера в дереве | Вместо символов `● ✕ ○` — значок `dot` цветом состояния (`--ok`, `--danger`, `--muted`) справа от имени, с прежней ролью изображения и подписью («подключён», «ошибка», «ещё не подключался») |
| Подвкладки сервера | Общий `SubTabs` Kubernetes-версии со значками: «Обзор» `grid`, «Конфигурация» `config`, «Метрики» `chart` |
| Символы и emoji | `✎ ⋯ ▸ ▾ 🔒 × ✕ ● ○ ★ ✅ ⚠ ❌ ⏳ +📁` заменены на `Icon`; `✅ ❌ ⚠` — компонент `StateMark`; `styles.test.ts` проверяет набор Kubernetes-версии плюс `○` |
| «Обновить» | `data-action="refresh"`, значок `refresh`, подсказка `⌘R`: «Подключить заново» на вкладке инсталляции, «Обновить» конфигурации сервера, «Обновить» метрик из истории, «Обновить» истории GPU. На экране в один момент видна одна подвкладка — первая кнопка видимой вкладки всегда та, что на экране |
| Фильтры | Только фильтр дерева (`.search-field` со значком `search`); ⌘F без фильтра на экране ставит фокус в него |
| Enter в формах | Не применяется: у экранов инсталляций нет формы с кнопкой запуска |
| Меню macOS | Как в Kubernetes-версии (`Menu.new`, не `Menu.default()` — ⌘W закрывает вкладку): File с «Новая инсталляция» (⌘N), View с «Фильтр инсталляций» (⇧⌘F), Help прежний: «Руководство Glafira» (⇧⌘/), «Поддержать проект», «О программе» |
| Сочетания | `shortcuts.ts` адаптирован: действия `newInstallation` (вкладка `installation-form`) и `filterInstallations`; остальное как есть |
| Окно | `titleBarStyle: Overlay`, `hiddenTitle`, `tauri-plugin-window-state`, разрешения `core:window:allow-start-dragging`, `core:window:allow-close`, `window-state:default` |
| Цвета окружений | `ENV_COLORS` на токены (как есть) |
| Словарь | `sidebar.emptyHint` без символа `⋯`: ссылается на меню «Ещё действия» по имени |
| Скриншоты | Dev-сервер Vite (порт 1440) + Chromium (Playwright из каталога сессии, не из репозитория) с заглушкой `window.__TAURI_INTERNALS__`; обе темы через `colorScheme` |

## 2. Дерево инсталляций: узлы и значки

```
Инсталляция      точка цвета (dot) · бейдж окружения · замок read-only · «Изменить» (pencil, при наведении и фокусе)
 ├ Обзор         grid          → вкладка installation
 ├ <сервер>      server        → вкладка server; справа — dot состояния (подключён / ошибка / ещё не подключался)
 └ GPU           chip          → вкладка gpu; подпись «GPU · N» по последнему снимку
```

Нижняя панель: «Настройки» (`settings`), «Поддержать проект» (`heart`), «Справка» (`book`), «О программе» (`info`) —
`TreeRow` без колонки шеврона. Индикатора MCP в этой версии нет (этап I2).

## 3. Значки (`Icon.tsx`)

Набор Kubernetes-версии как есть (в нём уже `server`, `config`, `chart`, `check`, `error`, `warning`, `hourglass`)
плюс значок этой версии (обводка 1.5, viewBox 16; расхождение отмечено в `PORTED.md`):

| Имя | Рисунок | Где |
|---|---|---|
| `chip` | квадрат с ножками по сторонам | «GPU» в дереве |

## 4. Переносимые файлы

Через `scripts/port.sh … … 41b611c` как есть: `src/styles.css`, `src/components/common/Icon.tsx` (затем дополнен §3),
`Icon.test.tsx`, `Kbd.tsx`, `Kbd.test.tsx`, `StateMark.tsx`, `SubTabs.tsx`, `SubTabs.test.tsx`, `EnvBadge.tsx`,
`Hint.tsx`, `Hint.test.tsx`, `src/components/connections/tree/TreeRow.tsx` → `src/components/installations/tree/TreeRow.tsx`,
`TreeRow.test.tsx`, `src/lib/platform.ts`, `src/lib/useResizable.ts`, `src/lib/env.ts`, `src/components/about/UpdateNotice.tsx`,
`vitest.config.ts`.

Адаптированы: `src/styles.test.ts` (глифы), `src/lib/shortcuts.ts` и `shortcuts.test.tsx` (инсталляции), `src/lib/useAppMenu.ts`
и `useAppMenu.test.ts` (пункты File и View, Help этой версии), `src/components/Layout.tsx`, `Sidebar.tsx`,
`installations/tree/InstallationTree.tsx`, `InstallationNode.tsx`, `installations/InstallationPanel.tsx`,
`servers/ServerPanel.tsx`, `ServerConfigView.tsx`, `ServerMetricsView.tsx`, `gpu/GpuPanel.tsx`, `GpuCardView.tsx`,
`GpuSummary.tsx`, `installations/InstallationForm.tsx`, `HistoryFields.tsx`, `src/app.css`, `src-tauri/tauri.conf.json`,
`src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, словари `en.json` и `ru.json`.

## 5. Тесты

Как есть: `Icon`, `Kbd`, `TreeRow`, `SubTabs`, `Hint`, `styles` (с `○`). Адаптированы: `shortcuts` (вкладка
`installation-form`), `useAppMenu` (пункты File и View, Help этой версии), `InstallationTree.test.tsx` (имена кнопок
через `aria-label`, состояние сервера — значок), `ServerPanel.test.tsx`, `InstallationForm.test.tsx` (значки состояния
вместо символов), `Sidebar.test.tsx`, `Layout.test.tsx`.
Проверка: `pnpm test`, `pnpm test:scripts`, `pnpm i18n:check`, `pnpm build`, `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`.

## 6. Вне объёма

Как в Kafka- и Kubernetes-версиях: палитра команд, изменяемая ширина панели деталей, пересмотр форм. Дополнительно:
индикатор MCP в боковой панели (этап I2), Enter в форме инсталляции («Проверить» и «Сохранить»).
