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

# ADR-0018. Parent commits that do not yet contain that ADR may also
# touch the contract files the ADR names. The list drops out once the
# ADR is in the baseline.
amendment_paths() {
  cat <<'EOF'
scripts/check-card-scope.sh
AGENTS.md
docs/adr/0011-dev-toolchain-and-nat-lab.md
docs/adr/0017-card-scope-gate.md
docs/adr/0018-card-status-line.md
docs/adr/README.md
docs/tasks/README.md
docs/spec.md
docs/dev.md
EOF
}

with_amendment() {
  local baseline=$1 allowed=$2
  if git cat-file -e "${baseline}:docs/adr/0018-card-status-line.md" 2>/dev/null; then
    printf '%s\n' "$allowed"
  else
    printf '%s\n%s\n' "$allowed" "$(amendment_paths)"
  fi
}

extract_statuses() {
  awk '
    /^### M[0-9]+-[0-9]+ / { id=$2; next }
    /^- \*\*Status:\*\*/ && id != "" { print id, $3 }
  '
}

roadmap_ticked() {
  local id=$1 roadmap_text=$2
  printf '%s\n' "$roadmap_text" | grep -Eq "^- \\[x\\] (\\*\\*)?${id}([[:space:]]|$)"
}

# 0 when a docs/tasks file differs from baseline only on allowed status lines.
card_status_edit_ok() {
  local file=$1 active=$2 baseline=$3 roadmap_text=$4 new_rev=$5
  local diff old new id old_st new_st bad
  [[ "$file" == docs/tasks/*.md ]] || return 1
  git cat-file -e "${baseline}:${file}" 2>/dev/null || return 1
  if [[ -n "$new_rev" ]]; then
    diff=$(git diff "${baseline}" "${new_rev}" -- "$file" || true)
    new=$(git show "${new_rev}:${file}" | extract_statuses)
  else
    diff=$(git diff "${baseline}" -- "$file" || true)
    new=$(extract_statuses < "$file")
  fi
  [[ -n "$diff" ]] || return 1
  bad=$(printf '%s\n' "$diff" | awk '
    /^(\+\+\+|---)/ { next }
    /^[+-]/ {
      line=substr($0, 2)
      if (line !~ /^- \*\*Status:\*\* (todo|done|blocked)$/) print line
    }
  ')
  [[ -z "$bad" ]] || return 1
  old=$(git show "${baseline}:${file}" | extract_statuses)
  while read -r id new_st; do
    [[ -z "$id" ]] && continue
    old_st=$(printf '%s\n' "$old" | awk -v id="$id" '$1==id { print $2; exit }')
    [[ "$old_st" == "$new_st" ]] && continue
    if [[ "$id" == "$active" ]]; then
      [[ "$new_st" == done || "$new_st" == blocked ]] || return 1
      continue
    fi
    if roadmap_ticked "$id" "$roadmap_text"; then
      [[ "$new_st" == done ]] || return 1
      continue
    fi
    return 1
  done <<< "$new"
}

check_files() {
  local id=$1 allowed_text=$2 changed_text=$3 baseline=$4 roadmap_text=$5 new_rev=$6
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
    if [[ "$matched" -eq 0 ]] && card_status_edit_ok "$file" "$id" "$baseline" "$roadmap_text" "$new_rev"; then
      matched=1
    fi
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
    allowed=$(with_amendment HEAD "$(allowlist_text "$id" HEAD)" | sort -u)
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
  check_files "$id" "$allowed" "$changed" HEAD "$(cat docs/roadmap.md)" ""
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
    allowed=$(with_amendment "${commit}^" "$(allowlist_text "$id" "${commit}^")" | sort -u)
    changed=$(git diff-tree --no-commit-id --name-only -r "$commit" | sed '/^$/d' | sort -u)
    check_files "$id" "$allowed" "$changed" "${commit}^" "$(git show "${commit}:docs/roadmap.md")" "$commit"
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
