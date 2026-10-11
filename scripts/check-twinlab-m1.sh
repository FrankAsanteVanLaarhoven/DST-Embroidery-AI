#!/usr/bin/env bash
# A real native acceptance gate. It never substitutes the fixture server.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
command -v cargo >/dev/null || { echo 'BLOCKED: cargo is required.' >&2; exit 2; }
command -v node >/dev/null || { echo 'BLOCKED: Node.js 22 or newer is required.' >&2; exit 2; }
node -e 'if(Number(process.versions.node.split(".")[0])<22)process.exit(2)'
export EMBROIDERY_PYTHON="${EMBROIDERY_PYTHON:-$ROOT/.venv/bin/python}"
[ -x "$EMBROIDERY_PYTHON" ] || { echo 'BLOCKED: install tests/twinlab/requirements.txt in .venv first.' >&2; exit 2; }
export PYTHONPATH="$ROOT/python${PYTHONPATH:+:$PYTHONPATH}"
mkdir -p evidence/native-gate
cargo test --workspace --locked 2>&1 | tee evidence/native-gate/cargo-test.log
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tee evidence/native-gate/clippy.log
cargo build --locked -p embroidery-ffi -p embroidery-twin 2>&1 | tee evidence/native-gate/build.log
"$EMBROIDERY_PYTHON" -m pytest python/tests -q 2>&1 | tee evidence/native-gate/pytest.log
node --test tests/twinlab/unit.mjs 2>&1 | tee evidence/native-gate/node-fixture.tap
node --test tests/twinlab/native.mjs 2>&1 | tee evidence/native-gate/node-native.tap
export PORT="${PORT:-4173}"
node apps/twinlab/server.mjs > evidence/native-gate/server.log 2>&1 &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT
sleep 1
kill -0 "$SERVER_PID" || { echo 'BLOCKED: local service did not start; check PORT.' >&2; exit 2; }
"$EMBROIDERY_PYTHON" tests/twinlab/browser.py --native --url "http://127.0.0.1:$PORT/" --output evidence/native-gate/browser
printf '\nNative gate passed for this run. Review its evidence and perform the manual checks in docs/twinlab-m1.md.\n'
