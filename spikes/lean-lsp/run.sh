#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 The lean4losophy Authors
#
# Run the M0 Lean LSP spike. Requires lake on PATH. Not part of the lint gate or
# CI: it needs a toolchain download and a real Lean build.

set -euo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd -- "$here"

if ! command -v lake >/dev/null 2>&1; then
  printf 'error: lake not found; install Lean 4 (%s) and retry\n' \
    "$(cat lean-toolchain)" >&2
  printf 'hint: elan can manage it: https://github.com/leanprover/elan\n' >&2
  exit 1
fi

printf 'info: toolchain %s\n' "$(cat lean-toolchain)"
exec python3 probe.py
