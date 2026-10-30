import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SCRIPT = new URL("./macos-notarize.sh", import.meta.url).pathname;
const skip = process.platform === "win32" && "скрипт для macOS: bash и подмена xcrun через PATH";

// Поддельный xcrun: `stapler staple` прикрепляет билет только с попытки STAPLE_OK_AT (0 — никогда) и всегда
// возвращает 0, как в сбое, который Tauri пропускает; `stapler validate` — есть ли билет. Вызовы — в calls.log.
const FAKE_XCRUN = `#!/usr/bin/env bash
set -eu
echo "$*" >> "$FAKE_DIR/calls.log"
[ "$1" = stapler ] || exit 1
case "$2" in
  validate) [ -e "$FAKE_DIR/stapled" ] ;;
  staple)
    n=$(( $(cat "$FAKE_DIR/count" 2>/dev/null || echo 0) + 1 ))
    echo "$n" > "$FAKE_DIR/count"
    if [ "$STAPLE_OK_AT" -gt 0 ] && [ "$n" -ge "$STAPLE_OK_AT" ]; then touch "$FAKE_DIR/stapled"; fi ;;
esac
`;

function run(args, { okAt = 1, stapled = false, attempts = 3 } = {}) {
  const dir = mkdtempSync(join(tmpdir(), "macos-notarize-"));
  const bin = join(dir, "bin");
  mkdirSync(bin);
  writeFileSync(join(bin, "xcrun"), FAKE_XCRUN);
  chmodSync(join(bin, "xcrun"), 0o755);
  if (stapled) writeFileSync(join(dir, "stapled"), "");
  const target = join(dir, "App.app");
  mkdirSync(target);
  const r = spawnSync("bash", [SCRIPT, ...(args ?? ["staple", target])], {
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${bin}:${process.env.PATH}`,
      FAKE_DIR: dir,
      STAPLE_OK_AT: String(okAt),
      STAPLE_ATTEMPTS: String(attempts),
      STAPLE_DELAY: "0",
    },
  });
  let calls = [];
  try {
    calls = readFileSync(join(dir, "calls.log"), "utf8").trim().split("\n");
  } catch {}
  return { ...r, calls, staples: calls.filter((c) => c.startsWith("stapler staple")).length, target };
}

test("билет уже есть — staple не вызывается", { skip }, () => {
  const r = run(undefined, { stapled: true });
  assert.equal(r.status, 0, r.stderr);
  assert.equal(r.staples, 0);
  assert.match(r.stdout, /билет уже прикреплён/);
});

test("билет появляется с третьей попытки — успех", { skip }, () => {
  const r = run(undefined, { okAt: 3, attempts: 5 });
  assert.equal(r.status, 0, r.stderr);
  assert.equal(r.staples, 3);
  assert.match(r.stdout, /попытка 3/);
});

test("staple «успешен», но validate билета не видит — повторы и ошибка, как бы ни отвечал staple", { skip }, () => {
  const r = run(undefined, { okAt: 0, attempts: 3 });
  assert.equal(r.status, 1);
  assert.equal(r.staples, 3);
  assert.match(r.stderr, /::error::не удалось прикрепить билет нотаризации за 3 попыток/);
});

test("неизвестная команда — подсказка и код 2", { skip }, () => {
  const r = run(["notarize-all", "/tmp"]);
  assert.equal(r.status, 2);
  assert.match(r.stderr, /использование/);
});

test("шаблон без совпадений или с несколькими — ошибка до вызова xcrun", { skip }, () => {
  const none = run(["app", "/nonexistent/*.app"]);
  assert.equal(none.status, 1);
  assert.match(none.stderr, /ожидался один существующий путь/);
  const two = run(["dmg", "/tmp", "/tmp"]);
  assert.equal(two.status, 1);
  assert.deepEqual([...none.calls, ...two.calls], []);
});
