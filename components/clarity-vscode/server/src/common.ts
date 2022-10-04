import {
  TextDocumentSyncKind,
  CompletionRequest,
  MarkupKind,
  InsertTextFormat,
} from "vscode-languageserver";
import type { ServerCapabilities, Connection } from "vscode-languageserver";

// this type is the same for the browser and node but node isn't alwasy built in dev
import type { LspVscodeBridge } from "./clarity-lsp-browser";

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
