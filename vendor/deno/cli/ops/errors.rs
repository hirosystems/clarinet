// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use crate::diagnostics::Diagnostics;
use crate::program_state::ProgramState;
use crate::source_maps::get_orig_position;
use crate::source_maps::CachedMaps;
use deno_core::error::AnyError;
use deno_core::serde_json;
use deno_core::serde_json::json;
use deno_core::serde_json::Value;
use deno_core::OpState;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

pub fn init(rt: &mut deno_core::JsRuntime) {
  super::reg_sync(rt, "op_apply_source_map", op_apply_source_map);
  super::reg_sync(rt, "op_format_diagnostic", op_format_diagnostic);
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplySourceMap {
  file_name: String,
  line_number: i32,
  column_number: i32,
}

fn op_apply_source_map(
  state: &mut OpState,
  args: Value,
  _: (),
) -> Result<String, AnyError> {
  let args: ApplySourceMap = serde_json::from_value(args)?;

  let mut mappings_map: CachedMaps = HashMap::new();
  let program_state = state.borrow::<Arc<ProgramState>>().clone();

  let (orig_file_name, orig_line_number, orig_column_number, _) =
    get_orig_position(
      args.file_name,
      args.line_number.into(),
      args.column_number.into(),
      &mut mappings_map,
      program_state,
    );

  Ok(
    json!({
      "fileName": orig_file_name,
      "lineNumber": orig_line_number as u32,
      "columnNumber": orig_column_number as u32,
    })
    .to_string(),
  )
}

fn op_format_diagnostic(
  _state: &mut OpState,
  args: Value,
  _: (),
) -> Result<Value, AnyError> {
  let diagnostic: Diagnostics = serde_json::from_value(args)?;
  Ok(json!(diagnostic.to_string()))
}
