#!/usr/bin/env bash
# Fail when a change sits outside the active card's **Files:** list.
# The card id comes from the committed Next task, not the worktree copy,
# so editing docs/progress.md cannot retarget the allowlist mid-session.
set -euo pipefail

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  echo "FAIL: not a git repository. The card scope gate compares git paths." >&2
  exit 1
fi
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

card_id_from_progress() {
  awk '
    /^## Next task/ { show=1; next }
    show && /^## / { exit }
    show && match($0, /M[0-9]+-[0-9]+/) {
      print substr($0, RSTART, RLENGTH)
      exit
    }
  '
}

card_file_at() {
  local id=$1 rev=$2 hit
  if [[ -n "$rev" ]]; then
    hit=$(git grep -l -e "^### ${id} " "$rev" -- docs/tasks 2>/dev/null || true)
    hit=${hit%%$'\n'*}
    hit=${hit#"${rev}:"}
    printf '%s\n' "$hit"
  else
    grep -l -E "^### ${id} " docs/tasks/*.md 2>/dev/null | head -n 1 || true
  fi
}

allowlist_text() {
  local id=$1 rev=$2 file
  file=$(card_file_at "$id" "$rev")
  if [[ -z "$file" ]]; then
    echo "FAIL: no card section '### ${id}' under docs/tasks/" >&2
    exit 1
  fi
  local raw paths
  if [[ -n "$rev" ]]; then
    raw=$(git show "${rev}:${file}")
  else
    raw=$(cat "$file")
  fi
  paths=$(printf '%s\n' "$raw" | awk -v id="$id" '
    index($0, "### " id " ") == 1 { in_card=1; next }
    in_card && index($0, "### ") == 1 { exit }
    in_card && index($0, "**Files:**") { print }
  ' | grep -oE '`[^`]+`' | tr -d '`' || true)
  if [[ -z "$paths" ]]; then
    echo "FAIL: card ${id} has no backtick paths on its **Files:** line" >&2
    exit 1
  fi
  printf '%s\n%s\n' "$paths" docs/progress.md
}

check_files() {
  local id=$1 allowed_text=$2 changed_text=$3
  local file allowed matched violations=0
  if [[ -z "$changed_text" ]]; then
    echo "card-scope-ok (${id})"
    return 0
  fi
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    matched=0
    while IFS= read -r allowed; do
      [[ -z "$allowed" ]] && continue
      if [[ "$allowed" == */ && "$file" == "$allowed"* ]]; then
        matched=1
        break
      fi
      if [[ "$file" == "$allowed" ]]; then
        matched=1
        break
      fi
    done <<< "$allowed_text"
    if [[ "$matched" -eq 0 ]]; then
      echo "SCOPE VIOLATION [${id}]: '${file}' is outside the card allowlist."
      violations=1
    fi
  done <<< "$changed_text"
  if [[ "$violations" -ne 0 ]]; then
    echo "Allowed paths for ${id}:"
    printf '%s\n' "$allowed_text" | sed '/^$/d' | sort -u | sed 's/^/  - /'
    exit 1
  fi
  echo "card-scope-ok (${id})"
}

check_worktree() {
  local progress id allowed changed
  if git rev-parse --verify HEAD >/dev/null 2>&1 \
    && git cat-file -e HEAD:docs/progress.md 2>/dev/null; then
    progress=$(git show HEAD:docs/progress.md)
    id=$(printf '%s\n' "$progress" | card_id_from_progress || true)
    if [[ -z "$id" ]]; then
      echo "FAIL: committed docs/progress.md has no Next task card id" >&2
      exit 1
    fi
    allowed=$(allowlist_text "$id" HEAD | sort -u)
  else
    id=$(card_id_from_progress < docs/progress.md || true)
    if [[ -z "$id" ]]; then
      echo "FAIL: No active card ID found in docs/progress.md" >&2
      exit 1
    fi
    allowed=$(allowlist_text "$id" "" | sort -u)
  fi
  changed=$(
    {
      git diff --name-only
      git diff --cached --name-only
      git ls-files --others --exclude-standard
    } | sed '/^$/d' | sort -u
  )
  check_files "$id" "$allowed" "$changed"
}

check_range() {
  local range=$1 commit parent id allowed changed skipped=0 checked=0
  if ! git rev-parse --verify "${range%%...*}" >/dev/null 2>&1; then
    echo "FAIL: cannot resolve range ${range}" >&2
    exit 1
  fi
  while IFS= read -r commit; do
    [[ -z "$commit" ]] && continue
    if ! git rev-parse --verify "${commit}^" >/dev/null 2>&1 \
      || ! git cat-file -e "${commit}^:scripts/check-card-scope.sh" 2>/dev/null; then
      skipped=$((skipped + 1))
      continue
    fi
    id=$(git show "${commit}^:docs/progress.md" | card_id_from_progress || true)
    if [[ -z "$id" ]]; then
      echo "FAIL: ${commit}^ has no Next task card id" >&2
      exit 1
    fi
    allowed=$(allowlist_text "$id" "${commit}^" | sort -u)
    changed=$(git diff-tree --no-commit-id --name-only -r "$commit" | sed '/^$/d' | sort -u)
    check_files "$id" "$allowed" "$changed"
    checked=$((checked + 1))
  done < <(git rev-list --reverse "$range")
  if [[ "$checked" -eq 0 ]]; then
    echo "card-scope-ok (introducing gate; skipped ${skipped} commit(s) whose parent lacks the script)"
  fi
}

if [[ "${1:-}" == "--range" ]]; then
  if [[ -z "${2:-}" ]]; then
    echo "FAIL: --range needs a git revision range" >&2
    exit 1
  fi
  check_range "$2"
else
  check_worktree
fi
