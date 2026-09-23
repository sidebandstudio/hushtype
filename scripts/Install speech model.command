#!/bin/bash
# Distributed beside setup-models.sh in the app download.
set -uo pipefail
script_dir="$(cd "$(dirname "$0")" && pwd)"
if /bin/bash "$script_dir/setup-models.sh" --model-only; then
  printf '\nOpen hushtype and choose Refresh models to finish setup.\n'
else
  printf '\nInstallation failed. Check the error above and run this installer again.\n' >&2
fi
read -r -p 'Press Return to close this window. ' ignored
