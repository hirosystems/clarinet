# Clarity VSCode Web Extension

The Clarity LSP web extension for VSCode.

## @TODO (about the extension, features, etc)

## Contributes

### Structure

The LSP has two main parts: the client and the server.
This two part will run differents environments:
- VSCode Web: in a WebWorkers
- VSCode Desktop: in a Node.js environments

The LSP, written in Rust, is built with wasm-pack for both these environments and will be load accordingly, with `fetch()` in the browser and `fs.readFileSync()` in Node.js.

```
.
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
