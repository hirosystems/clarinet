import * as vscode from "vscode";
import { emptyBody, insightsBody, head, getNonce } from "../utils/html";
import type { InsightsData } from "../types";

export class InsightsViewProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = "clarity-lsp.clarityInsightsView";

  private _view?: vscode.WebviewView;
  private _fileName?: string;
  private _insights?: InsightsData | null;

  constructor(private readonly _extensionUri: vscode.Uri) {}

  public set fileName(document: vscode.TextDocument) {
    this._fileName = document?.fileName;
    this._insights = null;
    if (!this._view) return;
    this._view.webview.html = this._getHtmlForWebview();
  }

  public set insights(data: InsightsData | null) {
    if (
      data &&
      this._insights?.fnName === data.fnName &&
      this._insights?.fnType === data.fnType
    ) {
      return;
    }

    this._insights = data;
    if (!this._view) return;
    this._view.webview.html = this._getHtmlForWebview();
  }

  public resolveWebviewView(webviewView: vscode.WebviewView) {
    this._view = webviewView;

    webviewView.webview.options = {
      enableScripts: false,
      localResourceRoots: [this._extensionUri],
    };

    webviewView.webview.html = this._getHtmlForWebview();
  }

  private _getHtmlForWebview() {
    const { _fileName: fileName, _insights: insights, _view: view } = this;
    const nonce = getNonce();
    if (!view) return "";

    const syleSrc = vscode.Uri.joinPath(
      view.webview.options.localResourceRoots![0],
      "./assets/styles/insightsView.css",
    );

    return /* html */ `<!DOCTYPE html>
      <html lang="en">
      ${head(view.webview, syleSrc, nonce)}

      <body>
        ${fileName ? /*html */ `<h4>${fileName}</h4>` : ""}

        ${insights ? insightsBody(insights) : emptyBody()}
      </body>
      </html>`;
  }
}
