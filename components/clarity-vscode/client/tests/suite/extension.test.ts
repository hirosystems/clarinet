import { assert } from "chai";
import {
  Uri,
  Range,
  workspace,
  window as vsWindow,
  languages,
  commands,
  DiagnosticSeverity,
} from "vscode";
import type { Diagnostic } from "vscode";

const { workspaceFolders } = workspace;
const { uri: workspaceUri } = workspaceFolders![0];

const delay = (ms: number) => new Promise((r) => setTimeout(() => r(1), ms));

beforeEach(() => {
  const config = workspace.getConfiguration("clarity-lsp");
  Object.keys(config).forEach((k) => {
    const setting = config.inspect(k);
    if (
      setting &&
      typeof setting.defaultValue !== "object" &&
      setting.defaultValue !== undefined
    ) {
      config.update(k, setting.defaultValue);
    }
  });
});

function getDiagnostics(uri: Uri) {
  let diagnosticPromise: (value: Diagnostic[]) => void;

  const waitForDiagnostic: Promise<Diagnostic[]> = new Promise(
    (resolve, reject) => {
      const timeout = setTimeout(() => reject("no diagnostic change"), 60_000);
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
    "test-cases/contracts/contract.clar",
  );

  it("get diagnostics on contract open", async function () {
    this.timeout(60_000);
    const diagnosticsListener = getDiagnostics(contractUri);

    await workspace.openTextDocument(contractUri);
    await delay(60000);
    const diagnostics = await diagnosticsListener;
    assert.strictEqual(diagnostics.length, 2);
    assert.strictEqual(diagnostics[0].severity, DiagnosticSeverity.Warning);
    assert.strictEqual(diagnostics[1].severity, DiagnosticSeverity.Information);
  });

  // it("get diagnostics on contract change", async () => {
  //   const document = await workspace.openTextDocument(contractUri);
  //   const editor = await vsWindow.showTextDocument(document, 1, false);

  //   const diagnosticsListener = getDiagnostics(contractUri);

  //   await editor.edit((editable) => {
  //     // uncomment line 9 of the contract
  //     editable.replace(new Range(8, 4, 8, 7), "");
  //   });

  //   const diagnostics = await diagnosticsListener;
  //   assert.strictEqual(diagnostics.length, 0);
  // });
});

// describe.only("get completion", function () {
//   this.timeout(20000);
//   afterEach(async () => {
//     commands.executeCommand("workbench.action.closeActiveEditor");
//   });

//   const contractUri: Uri = Uri.joinPath(
//     workspaceUri,
//     "test-cases/contracts/contract.clar",
//   );

//   it("show completion for native function", async () => {
//     const document = await workspace.openTextDocument(contractUri);
//     const editor = await vsWindow.showTextDocument(document, 1, false);

//     // waiting for the extension to be active
//     await getDiagnostics(contractUri);

//     await editor.edit(async (editable) => {
//       editable.insert(new Position(14, 0), "var-\n");
//     });

//     const position = editor.selection.active;
//     const newPosition = position.with(13, 4);
//     editor.selection = new Selection(newPosition, newPosition);
//     await commands.executeCommand("editor.action.triggerSuggest");
//     // todo
//   });
// });
