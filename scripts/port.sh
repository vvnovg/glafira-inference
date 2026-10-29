#!/usr/bin/env bash
# Переносит файл из Glafira for Kubernetes с зафиксированного коммита (строку о нём в PORTED.md добавляет задача плана).
# Использование: scripts/port.sh <путь в glafira-kubernetes> <путь здесь>
# Репозиторий Kubernetes-версии — $KUBERNETES_REPO (по умолчанию ~/projects/glafira-openshift).
set -euo pipefail
repo="${KUBERNETES_REPO:-$HOME/projects/glafira-openshift}"
commit=468e08a
src="$1"
dst="$2"
mkdir -p "$(dirname "$dst")"
git -C "$repo" show "$commit:$src" > "$dst"
