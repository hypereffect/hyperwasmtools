# List recipes
default:
    @just --list

# The apps come from the features of the package.
wasm_apps := `cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "hyperwasmtools") | (.features | keys | join(" "))'`

# Build one app as a wasm module into wasm-apps/, e.g. `just wasm-app worldclock`
wasm-app app:
    #!/usr/bin/env bash
    set -euo pipefail
    case " {{wasm_apps}} " in
        *" {{app}} "*) ;;
        *) echo "unknown app '{{app}}'; expected one of: {{wasm_apps}}" >&2; exit 1 ;;
    esac
    mkdir -p wasm-apps
    target=$(cargo metadata --no-deps --format-version 1 | jq -r .target_directory)
    cargo build -q --target wasm32-unknown-unknown --release -p hyperwasmtools --no-default-features --features "{{app}}"
    cp "$target/wasm32-unknown-unknown/release/hyperwasmtools.wasm" "wasm-apps/{{app}}.wasm"

# Build all the apps as wasm modules into wasm-apps/
wasm-apps:
    #!/usr/bin/env bash
    set -euo pipefail
    for app in {{wasm_apps}}; do
        just wasm-app "$app"
    done

# Serve wasm-apps/ over HTTP, with CORS for the web renderer, and print the apps' hyperb:// addresses
serve port="9001": wasm-apps
    #!/usr/bin/env python3
    import functools, http.server, pathlib
    for app in sorted(pathlib.Path("wasm-apps").glob("*.wasm")):
        print(f"hyperb://127.0.0.1:{{port}}/{app.name}")
    class Handler(http.server.SimpleHTTPRequestHandler):
        def end_headers(self):
            self.send_header("Access-Control-Allow-Origin", "*")
            super().end_headers()
    handler = functools.partial(Handler, directory="wasm-apps")
    http.server.ThreadingHTTPServer(("127.0.0.1", {{port}}), handler).serve_forever()

# Run a desktop hyperb renderer on an app, e.g. `just demo worldclock`
demo app="qr": wasm-apps
    hyperb-desktop "$PWD/wasm-apps/{{app}}.wasm"

# Run the tests
test:
    cargo test

# Remove build outputs
clean:
    cargo clean
    rm -rf wasm-apps
