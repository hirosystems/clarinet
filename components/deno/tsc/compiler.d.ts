// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

// Contains types that can be used to validate and check `99_main_compiler.js`

import * as _ts from "../dts/typescript";

declare global {
  // deno-lint-ignore no-namespace
  namespace ts {
    var libs: string[];
    var libMap: Map<string, string>;

    interface SourceFile {
      version?: string;
    }

    interface Performance {
      enable(): void;
      getDuration(value: string): number;
    }

    var performance: Performance;
  }

  // deno-lint-ignore no-namespace
  namespace ts {
    export = _ts;
  }

  interface Object {
    // deno-lint-ignore no-explicit-any
    __proto__: any;
  }

  interface DenoCore {
    // deno-lint-ignore no-explicit-any
    jsonOpSync<T>(name: string, params: T): any;
    ops(): void;
    print(msg: string, code?: number): void;
    registerErrorClass(
      name: string,
      Ctor: typeof Error,
      // deno-lint-ignore no-explicit-any
      ...args: any[]
    ): void;
  }

  type LanguageServerRequest =
    | ConfigureRequest
    | FindRenameLocationsRequest
    | GetAsset
    | GetCodeFixes
    | GetCombinedCodeFix
    | GetCompletionsRequest
    | GetDefinitionRequest
    | GetDiagnosticsRequest
    | GetDocumentHighlightsRequest
    | GetImplementationRequest
    | GetNavigationTree
    | GetQuickInfoRequest
    | GetReferencesRequest
    | GetSignatureHelpItemsRequest
    | GetSupportedCodeFixes;

  interface BaseLanguageServerRequest {
    id: number;
    method: string;
  }

  interface ConfigureRequest extends BaseLanguageServerRequest {
    method: "configure";
    // deno-lint-ignore no-explicit-any
    compilerOptions: Record<string, any>;
  }

  interface FindRenameLocationsRequest extends BaseLanguageServerRequest {
    method: "findRenameLocations";
    specifier: string;
    position: number;
    findInStrings: boolean;
    findInComments: boolean;
    providePrefixAndSuffixTextForRename: boolean;
  }

  interface GetAsset extends BaseLanguageServerRequest {
    method: "getAsset";
    specifier: string;
  }

  interface GetCodeFixes extends BaseLanguageServerRequest {
    method: "getCodeFixes";
    specifier: string;
    startPosition: number;
    endPosition: number;
    errorCodes: string[];
  }

  interface GetCombinedCodeFix extends BaseLanguageServerRequest {
    method: "getCombinedCodeFix";
    specifier: string;
    // deno-lint-ignore ban-types
    fixId: {};
  }

  interface GetCompletionsRequest extends BaseLanguageServerRequest {
    method: "getCompletions";
    specifier: string;
    position: number;
    preferences: ts.UserPreferences;
  }

  interface GetDiagnosticsRequest extends BaseLanguageServerRequest {
    method: "getDiagnostics";
    specifiers: string[];
  }

  interface GetDefinitionRequest extends BaseLanguageServerRequest {
    method: "getDefinition";
    specifier: string;
    position: number;
  }

  interface GetDocumentHighlightsRequest extends BaseLanguageServerRequest {
    method: "getDocumentHighlights";
    specifier: string;
    position: number;
    filesToSearch: string[];
  }

  interface GetImplementationRequest extends BaseLanguageServerRequest {
    method: "getImplementation";
    specifier: string;
    position: number;
  }

  interface GetNavigationTree extends BaseLanguageServerRequest {
    method: "getNavigationTree";
    specifier: string;
  }

  interface GetQuickInfoRequest extends BaseLanguageServerRequest {
    method: "getQuickInfo";
    specifier: string;
    position: number;
  }

  interface GetReferencesRequest extends BaseLanguageServerRequest {
    method: "getReferences";
    specifier: string;
    position: number;
  }

  interface GetSignatureHelpItemsRequest extends BaseLanguageServerRequest {
    method: "getSignatureHelpItems";
    specifier: string;
    position: number;
    options: ts.SignatureHelpItemsOptions;
  }

  interface GetSupportedCodeFixes extends BaseLanguageServerRequest {
    method: "getSupportedCodeFixes";
  }
}
