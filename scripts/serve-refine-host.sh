#!/bin/bash
# Installed by setup-refine-host.sh; launchd restarts this supervisor on exit.
set -euo pipefail

: "${HUSHTYPE_OLLAMA_BIN:?Missing Ollama executable}"
: "${HUSHTYPE_HOST_MODEL:?Missing local model}"
: "${OLLAMA_HOST:?Missing listening address}"
case "$HUSHTYPE_HOST_MODEL" in
  *[!a-zA-Z0-9./:_-]*|'') exit 64 ;;
esac

server_pid=''
awake_pid=''
cleanup() {
  trap - EXIT INT TERM
  [ -z "$awake_pid" ] || kill "$awake_pid" 2>/dev/null || true
  [ -z "$server_pid" ] || kill "$server_pid" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT
trap 'exit 0' INT TERM

"$HUSHTYPE_OLLAMA_BIN" serve &
server_pid=$!
# Prevent automatic system sleep on AC power while serving. This does not
# override lid closure, an explicit Sleep command, or FileVault login.
/usr/bin/caffeinate -s -w "$server_pid" &
awake_pid=$!

for ((attempt=0; attempt<60; attempt++)); do
  kill -0 "$server_pid" 2>/dev/null || wait "$server_pid"
  if /usr/bin/curl --noproxy '*' --silent --fail --max-time 2 \
    "http://$OLLAMA_HOST/api/version" >/dev/null; then
    if ! /usr/bin/curl --noproxy '*' --silent --show-error --fail --max-time 120 \
      --header 'Content-Type: application/json' \
      --data "{\"model\":\"$HUSHTYPE_HOST_MODEL\",\"stream\":false,\"keep_alive\":-1}" \
      "http://$OLLAMA_HOST/api/generate" >/dev/null; then
      printf 'Model preload failed; install %s on this host and restart the service.\n' "$HUSHTYPE_HOST_MODEL" >&2
    fi
    break
  fi
  sleep 1
done
wait "$server_pid"
