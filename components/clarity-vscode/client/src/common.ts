import * as vscode from "vscode";
import { ExtensionContext } from "vscode";
import { LanguageClientOptions } from "vscode-languageclient";

import { initVFS } from "./customVFS";
import { InsightsViewProvider } from "./Views/InsightsViewProvider";
import type { InsightsData, LanguageClient } from "./types";

const { window, workspace } = vscode;

function isValidInsight(data: InsightsData): data is InsightsData {
  return !!data && !!data.fnName && !!data.fnType && Array.isArray(data.fnArgs);
}

declare const __DEV_MODE__: boolean | undefined;

function getConfig() {
  const config = workspace.getConfiguration("clarity-lsp");
  if (__DEV_MODE__) {
    config.update("debug.logRequestsTimings", true);
  }
  return config;
}

export const clientOpts: LanguageClientOptions = {
  documentSelector: [{ language: "clarity" }, { language: "toml" }],
  diagnosticCollectionName: "Clarity LSP",
  progressOnInitialization: false,
  traceOutputChannel: vscode.window.createOutputChannel(
    "Clarity Language Server Trace",
  ),
  initializationOptions: JSON.stringify(getConfig()),
};

export async function initClient(
  context: ExtensionContext,
  client: LanguageClient,
) {
  if (__DEV_MODE__) {
    // update vscode default config in dev
    if (workspace.getConfiguration("files").autoSave !== "off") {
      vscode.commands.executeCommand("workbench.action.toggleAutoSave");
    }
    if (window.activeColorTheme.kind !== 2) {
      vscode.commands.executeCommand("workbench.action.toggleLightDarkThemes");
    }
  }

  let config = getConfig();
  const formatEnabled = config.get("format") === true;

  /* clarity insight webview */
  const insightsViewProvider = new InsightsViewProvider(context.extensionUri);

  context.subscriptions.push(
    window.registerWebviewViewProvider(
      InsightsViewProvider.viewType,
      insightsViewProvider,
    ),
  );

  workspace.onDidChangeConfiguration(async () => {
    let requireReload = false;
    const newConfig = getConfig();
    [
      "completion",
      "completionSmartParenthesisWrap",
      "completionIncludeNativePlaceholders",
      "format",
      "hover",
      "documentSymbols",
      "goToDefinition",
    ].forEach((k) => {
      if (newConfig[k] !== config[k]) requireReload = true;
    });

    config = newConfig;

    if (requireReload) {
      const userResponse = await vscode.window.showInformationMessage(
        "Changing Clarity configuration requires to reload VSCode",
        "Reload VSCode",
      );

      if (userResponse) {
        const command = "workbench.action.reloadWindow";
        await vscode.commands.executeCommand(command);
      }
    }
  });

  /* clariy lsp */
  async function changeSelectionHandler(
    e: vscode.TextEditorSelectionChangeEvent,
  ) {
    if (!e?.textEditor?.document) return;
    const path = e.textEditor.document.uri.toString();
    const { line, character } = e.selections[0].active;

    try {
      const res = await client.sendRequest("clarity/getFunctionAnalysis", {
        path,
        line: line + 1,
        char: character + 1,
      });
      if (!res) throw new Error("empty res");

      const insights = JSON.parse(res as string);
      if (!isValidInsight(insights)) throw new Error("Invalid insights");
      insightsViewProvider.insights = insights;
    } catch (err) {
      insightsViewProvider.insights = null;
      if (err instanceof Error && err.message === "empty res") return;
      console.warn(err);
    }
  }

  initVFS(client);
  try {
    await client.start();

    const now = new Date();
    const surveyStart = new Date("2023-05-12");
    const surveyEnd = new Date("2023-05-31");
    const surveyConfig = "showDevSurveyQ2-23-1";
    if (config.misc[surveyConfig] && now >= surveyStart && now <= surveyEnd) {
      window
        .showInformationMessage(
          "Help us improve Hiro products by telling us about your experience in a short 10 minute survey.",
          { title: "Take the survey", action: "open-survey" },
          { title: "No thanks", action: "hide" },
        )
        .then((v) => {
          if (v?.action === "hide" || v?.action === "open-survey") {
            console.log(v);
            config.update(`misc.${surveyConfig}`, false, true);
          }
          if (v?.action === "open-survey") {
            const surveyUri = vscode.Uri.parse("https://survey.hiro.so");
            vscode.commands.executeCommand("vscode.open", surveyUri);
          }
        });
    }

    const formatEnabled = config.get("format") === true;

    if (formatEnabled) {
      // register format-on-save if needed
      context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider("clarity", {
          provideDocumentFormattingEdits: (document, options, _token) => {
            // Only format clarity documents
            if (document.languageId !== "clarity") return null;

            // Get formatting edits from the server
            const params = {
              textDocument: {
                uri: document.uri.toString(),
              },
              options: {
                tabSize: options.tabSize,
                insertSpaces: options.insertSpaces,
                trimTrailingWhitespace: vscode.workspace
                  .getConfiguration("files")
                  .get("trimTrailingWhitespace", false),
                trimFinalNewlines: vscode.workspace
                  .getConfiguration("files")
                  .get("trimFinalNewlines", false),
                insertFinalNewline: vscode.workspace
                  .getConfiguration("files")
                  .get("insertFinalNewline", false),
              },
            };

            return client.sendRequest("textDocument/formatting", params);
          },
        }),
      );

      // Listen for document save and format if enabled
      context.subscriptions.push(
        vscode.workspace.onWillSaveTextDocument((event) => {
          if (event.document.languageId === "clarity") {
            const formatPromise = vscode.commands.executeCommand(
              "editor.action.formatDocument",
            );
            event.waitUntil(formatPromise);
          }
        }),
      );
    }

    if (config.panels["insights-panel"]) {
      if (window.activeTextEditor) {
        const { document } = window.activeTextEditor;
        if (document.languageId !== "clarity") return;
        insightsViewProvider.fileName = document;
      }

      context.subscriptions.push(
        window.onDidChangeTextEditorSelection(changeSelectionHandler),
      );
    }
  } catch (err) {
    if (err instanceof Error && err.message === "worker timeout") {
      vscode.window.showWarningMessage(
        "Clarity Language Server failed to start",
      );
    }
  }
}
