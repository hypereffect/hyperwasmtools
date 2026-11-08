# hyperwasmtools

Small, useful tools that run as sandboxed WASM modules in [hyperb](https://github.com/hypereffect/hyperb) renderers. They need no server: each one runs locally inside the renderer.

| App | What it does |
| --- | --- |
| [`qr`](wasm-apps/qr.wasm) | Generate QR codes |
| [`textutils`](wasm-apps/textutils.wasm) | A pocket knife of small tools for text, data, numbers and more |
| [`worldclock`](wasm-apps/worldclock.wasm) | Live clocks across time zones, and a time converter |
| [`finance`](wasm-apps/finance.wasm) | An RPN financial calculator |
| [`billsplit`](wasm-apps/billsplit.wasm) | Split a bill |
| [`wordle`](wasm-apps/wordle.wasm) | Guess the five-letter word in six tries |

## Usage

Open a module with any hyperb renderer, for example:

```sh
hyperb-desktop "$PWD/wasm-apps/qr.wasm"
```

To serve the modules over HTTP with CORS, and print their `hyperb://` addresses:

```sh
just serve
```

## Building

Each app is built separately for `wasm32-unknown-unknown`, with one feature selected:

```sh
rustup target add wasm32-unknown-unknown
just wasm-app qr
just wasm-apps        # all of them
```

The modules are written to `wasm-apps/`. Run the native tests with `just test`.

## License

MIT OR Apache-2.0.
