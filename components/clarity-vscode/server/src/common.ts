import {
  CompletionRequest,
  DidOpenTextDocumentParams,
  DidCloseTextDocumentNotification,
  DidOpenTextDocumentNotification,
  InitializeRequest,
} from "vscode-languageserver";
import type { Connection } from "vscode-languageserver";

// this type is the same for the browser and node but node isn't alwasy built in dev
import type { LspVscodeBridge } from "./clarity-lsp-browser/lsp-browser";

const VALID_PROTOCOLS = ["file", "vscode-vfs", "vscode-test-web"];

export function initConnection(
  connection: Connection,
  bridge: LspVscodeBridge,
) {
  connection.onInitialize(async (params) =>
    bridge.onRequest(InitializeRequest.method, params),
  );

  const notifications: [string, unknown][] = [];
  async function consumeNotification() {
    const notification = notifications[notifications.length - 1];
    if (!notifications) return;
    try {
      await bridge.onNotification(...notification);
    } catch (err) {
      console.warn(err);
    }
    notifications.pop();
    if (notifications.length > 0) consumeNotification();
  }

  connection.onNotification((method: string, params: unknown) => {
    // vscode.dev sends didOpen notification twice
    // including a notification with a read only github:// url
    // instead of vscode-vfs://
    if (
      method === DidOpenTextDocumentNotification.method ||
      method === DidCloseTextDocumentNotification.method
    ) {
      const [protocol] = (
        params as DidOpenTextDocumentParams
      ).textDocument.uri.split("://");
      if (!VALID_PROTOCOLS.includes(protocol)) return;
    }

    notifications.push([method, params]);
    if (notifications.length === 1) consumeNotification();
  });

  connection.onRequest(async (method: string, params: unknown) => {
    return bridge.onRequest(method, params);
  });

  connection.onCompletion(async (params: unknown) => {
    // notifications and requests are competing to get access to the editor_state_lock
    // in the (occasional) event of a completion request happening while the server has
    // notifications to handle, let's ignore the completion request
    if (notifications.length > 0) return null;
    return bridge.onRequest(CompletionRequest.method, params);
  });

  connection.listen();
}
