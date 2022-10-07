# Clarity for VSCode

Clarity is a decidable smart contract language that optimizes for predictability and security. Smart contracts allow developers to encode essential business logic on a blockchain.

This extension aims to help developer write safe and clean Clarity code, it includes syntax highlighting, auto-completion, linting, debugging and [safety checks](https://www.hiro.so/blog/new-safety-checks-in-clarinet),.

## Support for [vscode.dev](https://vscode.dev)

The Clarity extension runs in the browser so it can be used with [vscode.dev](https://vscode.dev) and [github.dev](https://github.dev), the same way it runs in VSCode for Desktop.

The Debugger is not implemented yet on the browser.

## Debugging

The debugger included in this extension allows to run Clarity code line by line which is a powerful tool to understand what precisely happens. Read this [blog post](https://www.hiro.so/blog/how-to-debug-your-smart-contracts-with-clarinet) to know more about Clarity debugging.

In order to use the Debugger (DAP), this extension relies on a local installation of Clarinet. To install Clarinet, please follow the instructions [here](https://github.com/hirosystems/clarinet#installation).

---
## Contributes

### Run the extension locally

You'll need to have Rust, Node.js and NPM installed.

From the `./components/clarity-vscode`, run `npm install` to install the dependencies and `npm run dev` to start the extension in a Chromium instance.

### Structure

The LSP has two main parts: the client and the server.
This two part will run differents environments:
- VSCode Web: in a WebWorkers
- VSCode Desktop: in a Node.js environments

The LSP (`./components/clarity-lsp`), written in Rust, is built with wasm-pack for both these environments and will be load accordingly, with `fetch()` in the browser and `fs.readFileSync()` in Node.js.

```
./components/clarity-vscode
├── package.json // The extension manifest.
├── client
│   └── src
│       ├── clientBrowser.ts
│       ├── clientNode.ts
│       └── common.ts
└── server
    └── src
        ├── serverBrowser.ts
        ├── serverNode.ts
        └── common.ts
```
