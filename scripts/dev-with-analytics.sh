#!/usr/bin/env bash
#
# Launch `npm run tauri dev` with the Aptabase analytics plugin compiled in,
# pointing at the dedicated "openehr-explorer-dev" Aptabase project so dev
# clicks don't pollute production metrics (see ADR-0018).
#
# Usage:
#
#     npm run dev:analytics
#
# One-time setup:
#
#     1. Create a second app in Aptabase (e.g. "openehr-explorer-dev").
#     2. Copy its App Key (looks like A-EU-xxxxxxxxxx or A-US-xxxxxxxxxx).
#     3. Create `.env.analytics.local` at the repo root with:
#
#            APTABASE_APP_KEY=A-EU-xxxxxxxxxx
#
#        `.env.analytics.local` matches `*.local` in .gitignore so it is
#        never committed.
#
# If the file (or the key) is missing the script still runs, but the Aptabase
# plugin sees an empty key and disables itself — every `track_event` call
# becomes a no-op. That means `npm run dev:analytics` is safe for new
# contributors who haven't provisioned a key yet.
#
# To actually see events land in the Aptabase dashboard you also need to
# toggle the opt-in switch at Settings -> Analytics inside the running app.

set -euo pipefail

# Resolve repo root relative to this script so the command works regardless
# of where it is invoked from (npm scripts set cwd to the package root, but
# running the script directly should also work).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="$REPO_ROOT/.env.analytics.local"

if [ -f "$ENV_FILE" ]; then
  # shellcheck disable=SC1090
  set -a
  source "$ENV_FILE"
  set +a
  echo "[dev-with-analytics] loaded $ENV_FILE"
else
  echo "[dev-with-analytics] no $ENV_FILE found — Aptabase plugin will"
  echo "[dev-with-analytics] be compiled in with an empty key (disabled)."
  echo "[dev-with-analytics] See ADR-0018 for setup instructions."
fi

# APTABASE_APP_KEY is read by `option_env!` in src-tauri/src/lib.rs at rustc
# compile time. Export it so the child cargo process inherits it; fall back
# to empty string so the plugin cleanly disables itself.
export APTABASE_APP_KEY="${APTABASE_APP_KEY:-}"

# Show gating decisions in the terminal so it's obvious whether events are
# actually being enqueued. Can be overridden by setting RUST_LOG yourself.
export RUST_LOG="${RUST_LOG:-tauri_plugin_aptabase=debug}"

# `option_env!` is evaluated during compilation and cached by cargo's
# incremental build cache. Changing APTABASE_APP_KEY without rebuilding the
# crate silently reuses the stale value — force the Tauri lib crate to be
# recompiled so whatever is in the env var right now is what gets baked in.
echo "[dev-with-analytics] forcing rebuild of openehr-explorer so the key is re-baked"
cargo clean -p openehr-explorer --manifest-path "$REPO_ROOT/src-tauri/Cargo.toml" >/dev/null 2>&1 || true

if [ -n "$APTABASE_APP_KEY" ]; then
  echo "[dev-with-analytics] APTABASE_APP_KEY is set (${APTABASE_APP_KEY:0:6}…) — analytics enabled at build time"
else
  echo "[dev-with-analytics] APTABASE_APP_KEY is empty — plugin will be a no-op"
fi

echo "[dev-with-analytics] remember to toggle Settings → Analytics ON inside the app"

cd "$REPO_ROOT"
exec npm run tauri dev
