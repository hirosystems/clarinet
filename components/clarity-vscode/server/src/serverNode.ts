import { createConnection } from "vscode-languageserver/node";
import fetch, { Headers, Request, Response } from "node-fetch";

import { LspVscodeBridge } from "./clarity-lsp-node";
import { initConnection } from "./common";

// wasm-pack needs the node-fetch polyfill when targetting node.js
// https://rustwasm.github.io/wasm-pack/book/prerequisites/considerations.html
// @ts-ignore
global.fetch = fetch;
// @ts-ignore
global.Headers = Headers;
// @ts-ignore
global.Request = Request;
// @ts-ignore
global.Response = Response;

const connection = createConnection();
const bridge = new LspVscodeBridge(
  connection.sendDiagnostics,
  connection.sendNotification,
  connection.sendRequest,
);

initConnection(connection, bridge);
