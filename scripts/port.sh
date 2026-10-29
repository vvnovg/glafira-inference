#!/usr/bin/env bash
# Переносит файл из Glafira for Kubernetes с зафиксированного коммита (строку о нём в PORTED.md добавляет задача плана).
# Использование: scripts/port.sh <путь в glafira-kubernetes> <путь здесь>
# Репозиторий Kubernetes-версии — $KUBERNETES_REPO (по умолчанию ~/projects/glafira-openshift).
set -euo pipefail
if [ "$#" -ne 2 ]; then
  echo "usage: scripts/port.sh <путь в glafira-kubernetes> <путь здесь>" >&2
  exit 2
fi
repo="${KUBERNETES_REPO:-$HOME/projects/glafira-openshift}"
commit=468e08a
src="$1"
dst="$2"
mkdir -p "$(dirname "$dst")"
tmp="$dst.tmp.$$"
trap 'rm -f "$tmp"' EXIT
git -C "$repo" show "$commit:$src" > "$tmp"
mv "$tmp" "$dst"
