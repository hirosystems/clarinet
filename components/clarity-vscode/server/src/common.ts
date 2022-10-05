import {
  TextDocumentSyncKind,
  CompletionRequest,
  MarkupKind,
  InsertTextFormat,
  DidOpenTextDocumentParams,
  DidCloseTextDocumentNotification,
  DidOpenTextDocumentNotification,
} from "vscode-languageserver";
import type { ServerCapabilities, Connection } from "vscode-languageserver";

// this type is the same for the browser and node but node isn't alwasy built in dev
import type { LspVscodeBridge } from "./clarity-lsp-browser";

const VALID_PROTOCOLS = ["file", "vscode-vfs", "vscode-test-web"];

export function initConnection(
  connection: Connection,
  bridge: LspVscodeBridge,
) {
  connection.onInitialize(() => {
    const capabilities: ServerCapabilities = {
      textDocumentSync: {
        change: TextDocumentSyncKind.None,
        willSave: false,
        openClose: true,
        save: { includeText: false },
      },
      completionProvider: {},
    };
    return { capabilities };
  });

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
    if (method === DidOpenTextDocumentNotification.method) {
      const [protocol] = (
        params as DidOpenTextDocumentParams
      ).textDocument.uri.split("://");

      if (!VALID_PROTOCOLS.includes(protocol)) return;
    } else if (method === DidCloseTextDocumentNotification.method) {
      // ignore didClose notifications
      // it fires twice as well for nothing
      return;
    }

    notifications.push([method, params]);
    if (notifications.length === 1) consumeNotification();
  });

  connection.onRequest(async (method: string, params: unknown) => {
    return bridge.onRequest(method, params);
  });

  connection.onCompletion(async (params: unknown) => {
    const res = await bridge.onRequest(CompletionRequest.method, params);
    if (!res) return null;
    return res.map((item: Record<string, any>) => ({
      ...item,
      insertText: item.insert_text,
      insertTextFormat:
        item.insert_text_format === "PlainText"
          ? InsertTextFormat.PlainText
          : InsertTextFormat.Snippet,
      documentation: {
        kind: MarkupKind.Markdown,
        value: item.markdown_documentation,
      },
    }));
  });

  connection.listen();
}
