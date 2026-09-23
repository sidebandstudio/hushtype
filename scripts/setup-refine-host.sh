#!/bin/bash
# Configure an existing Ollama installation as an optional macOS refinement host.
set -euo pipefail

if [ "$(uname -s)" != Darwin ]; then
  printf 'This setup script supports macOS.\n' >&2
  exit 1
fi
host_ip="${1:-$(tailscale ip -4)}"
host_model="${2:-qwen3:8b}"
case "$host_ip" in
  100.*) ;;
  *) printf 'Pass this Mac’s Tailscale IPv4 address (tailscale ip -4).\n' >&2; exit 1 ;;
esac
case "$host_ip" in *[!0-9.]*|'') exit 64 ;; esac
case "$host_model" in *[!a-zA-Z0-9./:_-]*|'') exit 64 ;; esac
ollama_bin="$(command -v ollama)"
script_dir="$(cd "$(dirname "$0")" && pwd)"
service_dir="$HOME/Library/Application Support/dev.hushtype.host"
log_dir="$HOME/Library/Logs/dev.hushtype.host"
plist="$HOME/Library/LaunchAgents/dev.hushtype.ollama.plist"
label=dev.hushtype.ollama
domain="gui/$(id -u)"
mkdir -p "$service_dir" "$log_dir" "$(dirname "$plist")"
cp "$script_dir/serve-refine-host.sh" "$service_dir/serve.sh"
chmod 700 "$service_dir/serve.sh"

xml() { printf '%s' "$1" | sed -e 's/\&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'; }
cat > "$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>$label</string>
  <key>ProgramArguments</key><array><string>/bin/bash</string><string>$(xml "$service_dir/serve.sh")</string></array>
  <key>EnvironmentVariables</key><dict>
    <key>HUSHTYPE_OLLAMA_BIN</key><string>$(xml "$ollama_bin")</string>
    <key>HUSHTYPE_HOST_MODEL</key><string>$(xml "$host_model")</string>
    <key>OLLAMA_HOST</key><string>$host_ip:11434</string>
    <key>OLLAMA_NO_CLOUD</key><string>1</string>
    <key>OLLAMA_KEEP_ALIVE</key><string>-1</string>
    <key>OLLAMA_FLASH_ATTENTION</key><string>1</string>
    <key>OLLAMA_CONTEXT_LENGTH</key><string>4096</string>
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ThrottleInterval</key><integer>10</integer>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardOutPath</key><string>$(xml "$log_dir/server.log")</string>
  <key>StandardErrorPath</key><string>$(xml "$log_dir/server.log")</string>
</dict></plist>
EOF
chmod 600 "$plist"
plutil -lint "$plist"
if launchctl print "$domain/$label" >/dev/null 2>&1; then
  launchctl bootout "$domain/$label"
fi
loaded=false
for ((attempt=0; attempt<15; attempt++)); do
  if bootstrap_error="$(launchctl bootstrap "$domain" "$plist" 2>&1)"; then
    loaded=true
    break
  fi
  # bootout returns before the previous process has necessarily exited.
  sleep 1
done
if [ "$loaded" != true ]; then
  printf '%s\n' "$bootstrap_error" >&2
  exit 1
fi
printf 'Refine base URL: http://%s:11434/v1\nModel: %s\nLogs: %s/server.log\n' "$host_ip" "$host_model" "$log_dir"
printf 'The service starts at login. Keep this host powered and signed in.\n'
