#!/usr/bin/env python3
"""Assemble the GitHub Pages site from the built WASM modules.

Usage: build-site.py <output-dir> <base-url>
"""

import html
import json
import pathlib
import shutil
import sys

DESCRIPTIONS = {
    "billsplit": "Split a bill",
    "finance": "An RPN financial calculator",
    "qr": "Generate QR codes",
    "textutils": "A pocket knife of small tools for text, data, numbers and more",
    "wordle": "Guess the five-letter word in six tries",
    "worldclock": "Live clocks across time zones, and a time converter",
}


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <output-dir> <base-url>", file=sys.stderr)
        return 2

    output_dir = pathlib.Path(sys.argv[1])
    base_url = sys.argv[2].rstrip("/")
    wasm_dir = pathlib.Path("wasm-apps")

    modules = sorted(wasm_dir.glob("*.wasm"), key=lambda path: path.stem)
    if not modules:
        print(f"no WASM modules found in {wasm_dir}", file=sys.stderr)
        return 1

    published_dir = output_dir / "wasm-apps"
    published_dir.mkdir(parents=True, exist_ok=True)

    apps = []
    for module in modules:
        name = module.stem
        shutil.copy2(module, published_dir / module.name)
        apps.append(
            {
                "name": name,
                "description": DESCRIPTIONS.get(name, ""),
                "url": f"{base_url}/wasm-apps/{module.name}",
                "wasm": f"wasm-apps/{module.name}",
            }
        )

    (output_dir / "apps.json").write_text(
        json.dumps(apps, indent=2) + "\n", encoding="utf-8"
    )
    (output_dir / ".nojekyll").write_text("", encoding="utf-8")

    rows = "\n".join(
        f"""        <tr>
          <td><a href="{html.escape(app["wasm"])}"><strong>{html.escape(app["name"])}</strong></a></td>
          <td>{html.escape(app["description"])}</td>
          <td><a href="{html.escape(app["url"])}">download</a></td>
        </tr>"""
        for app in apps
    )

    page = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>hyperwasmtools</title>
  <style>
    :root {{
      color-scheme: light dark;
      font-family: system-ui, sans-serif;
      line-height: 1.5;
    }}
    body {{
      margin: 0 auto;
      max-width: 52rem;
      padding: 2rem 1rem;
    }}
    h1 {{ margin-bottom: 0.25rem; }}
    .intro {{ margin-top: 0; }}
    table {{ border-collapse: collapse; width: 100%; }}
    th, td {{ padding: 0.5rem; text-align: left; border-bottom: 1px solid #8884; }}
    code {{ overflow-wrap: anywhere; }}
  </style>
</head>
<body>
  <h1>hyperwasmtools</h1>
  <p class="intro">
    Small, useful tools that run as sandboxed WASM modules in
    <a href="https://github.com/hypereffect/hyperb">hyperb</a> renderers.
    They need no server: each one runs locally inside the renderer.
  </p>
  <p>
    Open a module with any hyperb renderer, for example
    <code>hyperb-desktop &lt;url&gt;</code>. A machine-readable list of the
    modules is available in <a href="apps.json">apps.json</a>.
  </p>
  <table>
    <thead>
      <tr><th>App</th><th>Description</th><th>Module</th></tr>
    </thead>
    <tbody>
{rows}
    </tbody>
  </table>
</body>
</html>
"""

    (output_dir / "index.html").write_text(page, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
