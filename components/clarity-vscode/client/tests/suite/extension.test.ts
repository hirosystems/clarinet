import { assert } from "chai";
import {
  Uri,
  Range,
  workspace,
  window,
  languages,
  commands,
  DiagnosticSeverity,
} from "vscode";
import type { Diagnostic } from "vscode";

const { workspaceFolders } = workspace;
const { uri: workspaceUri } = workspaceFolders![0];

const delay = (ms: number) => new Promise((r) => setTimeout(() => r(1), ms));

function getDiagnostics(uri: Uri) {
  let diagnosticPromise: (value: Diagnostic[]) => void;

  const waitForDiagnostic: Promise<Diagnostic[]> = new Promise(
    (resolve, reject) => {
      const timeout = setTimeout(() => reject("no diagnostic change"), 4000);
      diagnosticPromise = (value) => {
        clearTimeout(timeout);
        resolve(value);
      };
    },
  );

  const disposable = languages.onDidChangeDiagnostics(() => {
    diagnosticPromise(languages.getDiagnostics(uri));
  });
  waitForDiagnostic.finally(() => disposable.dispose());

  return waitForDiagnostic;
}

describe("get diagnostics", () => {
  afterEach(() => {
    commands.executeCommand("workbench.action.closeActiveEditor");
  });

  const contractUri: Uri = Uri.joinPath(
    workspaceUri,
    "with-errors/contracts/contract.clar",
  );

  it("get diagnostics on contract open", async () => {
    const diagnosticsListener = getDiagnostics(contractUri);

    await workspace.openTextDocument(contractUri);
    const diagnostics = await diagnosticsListener;
    assert.strictEqual(diagnostics.length, 2);
    assert.strictEqual(diagnostics[0].severity, DiagnosticSeverity.Warning);
    assert.strictEqual(diagnostics[1].severity, DiagnosticSeverity.Information);
  });

  it("get diagnostics on contract change", async () => {
    const document = await workspace.openTextDocument(contractUri);
    const editor = await window.showTextDocument(document, 1, false);

    const diagnosticsListener = getDiagnostics(contractUri);

    editor.edit((editable) => {
      // uncomment line 9 of the contract
      editable.replace(new Range(8, 4, 8, 7), "");
    });

    const diagnostics = await diagnosticsListener;
    assert.strictEqual(diagnostics.length, 0);
  });
});
