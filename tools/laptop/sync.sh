#!/usr/bin/env bash
# Run on the desktop (Git Bash) from the repo root: copy the committed state of master to the laptop.
# Uncommitted changes are not sent. The laptop copy is overwritten to match.
set -euo pipefail
HOST=USER@LAPTOP_HOST
OPTS=(-i "$HOME/.ssh/id_ed25519" -o IdentitiesOnly=yes -o BatchMode=yes -o ConnectTimeout=8)
REMOTE=C:/Users/grich/Code/pcsa
bundle="$(mktemp -d)/champions-gym.bundle"
git bundle create -q "$bundle" master
scp -q "${OPTS[@]}" "$bundle" "$HOST:$REMOTE/champions-gym.bundle"
scp -q "${OPTS[@]}" tools/laptop/laptop_sync.cmd "$HOST:$REMOTE/champions-gym-sync.cmd"
ssh "${OPTS[@]}" "$HOST" 'C:\Users\grich\Code\pcsa\champions-gym-sync.cmd'
rm -f "$bundle"
