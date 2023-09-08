use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::convert::TryFrom;

use clar2wasm::{compile, CompileResult};
use clarity::types::StacksEpochId;
use clarity::vm::contexts::GlobalContext;
use clarity::vm::types::{BufferLength, SequenceSubtype, StringSubtype};
use clarity::vm::{
    analysis::ContractAnalysis,
    costs::LimitedCostTracker,
    database::MemoryBackingStore,
    types::{FunctionType, QualifiedContractIdentifier, TypeSignature},
    ClarityVersion,
};
use clarity::vm::{ClarityName, ContractContext, Value};
use wasmtime::{
    AsContextMut, Caller, Engine, FuncType, Instance, Linker, Module, Store, Val, ValType,
};

pub struct ClarityWasmContext<'a, 'b> {
    /// The global context in which to execute.
    pub global_context: &'b mut GlobalContext<'a>,
    /// Context for this contract. This will be filled in when running the
    /// top-level expressions, then used when calling functions.
    pub contract_context: &'b mut ContractContext,
    /// The contract analysis for the compiled contract
    pub contract_analysis: ContractAnalysis,
    /// Map an identifier from a contract to an integer id for simple access
    pub identifier_map: HashMap<i32, String>,
}

impl<'a, 'b> ClarityWasmContext<'a, 'b> {
    pub fn new(
        global_context: &'b mut GlobalContext<'a>,
        contract_context: &'b mut ContractContext,
        contract_analysis: ContractAnalysis,
    ) -> Self {
        ClarityWasmContext {
            global_context,
            contract_analysis,
            contract_context,
            identifier_map: HashMap::new(),
        }
    }
}

/// A simple wrapper for WASMTime to help reduce the amount of boilerplate needed
/// in test code. The wrapper compiles the specified contract using `clar2wasm` and
/// stores a copy of its contract analysis for type inferrence when calling functions.
pub struct WasmtimeHelper<'a, 'b> {
    instance: Instance,
    store: Box<Store<ClarityWasmContext<'a, 'b>>>,
}

/// Generates a WASMTime function signature (both input and return arguments), provided the
/// given Clarity `FunctionType`.
fn generate_wasmtime_func_signature(fn_sig: &FunctionType) -> FuncType {
    let mut params = Vec::<ValType>::new();
    let mut returns = Vec::<ValType>::new();

    match fn_sig {
        FunctionType::Fixed(func) => {
            for arg in func.args.iter() {
                let mut arg_sig = get_wasmtime_arg(&arg.signature);
                params.append(&mut arg_sig);
            }

            let mut returns_sig = get_wasmtime_arg(&func.returns);
            returns.append(&mut returns_sig);
        }
        _ => panic!("Not implemented"),
    }

    FuncType::new(params, returns)
}

/// Creates the type signature expected by WASMTime for the provided Clarity `TypeSignature`.
fn get_wasmtime_arg(type_sig: &TypeSignature) -> Vec<ValType> {
    match type_sig {
        TypeSignature::IntType | TypeSignature::UIntType => vec![ValType::I64, ValType::I64],
        TypeSignature::BoolType => vec![ValType::I32],
        TypeSignature::SequenceType(_) => vec![ValType::I32, ValType::I32],
        TypeSignature::ResponseType(resp) => {
            let mut sig = vec![ValType::I32; 1];
            let mut ok_type = get_wasmtime_arg(&resp.0);
            let mut err_type = get_wasmtime_arg(&resp.1);
            sig.append(&mut ok_type);
            sig.append(&mut err_type);
            sig
        }
        TypeSignature::NoType => vec![ValType::I32],
        _ => panic!("Not implemented"),
    }
}

/// Maps the result from a WASM function call given the provided Clarity `FunctionType`.
fn map_wasm_result(fn_sig: &FunctionType, result: &[Val]) -> Value {
    match fn_sig {
        FunctionType::Fixed(func) => {
            let (result, _) = map_wasm_value(&func.returns, 0, result);
            result
        }
        _ => panic!("Function type '{:?}' not implemented.", &fn_sig),
    }
}

/// Maps an individual value in a WASM function call result.
fn map_wasm_value(type_sig: &TypeSignature, index: usize, buffer: &[Val]) -> (Value, usize) {
    match type_sig {
        TypeSignature::IntType => {
            let upper = buffer[index].unwrap_i64();
            let lower = buffer[index + 1].unwrap_i64();
            (Value::Int(((upper as i128) << 64) | lower as i128), 2)
        }
        TypeSignature::UIntType => {
            let upper = buffer[index].unwrap_i64();
            let lower = buffer[index + 1].unwrap_i64();
            (Value::UInt(((upper as u128) << 64) | lower as u128), 2)
        }
        TypeSignature::BoolType => (Value::Bool(buffer[index].unwrap_i32() != 0), 1),
        TypeSignature::OptionalType(optional) => {
            let (value, increment) = map_wasm_value(optional, index + 1, buffer);
            (
                if buffer[index].unwrap_i32() == 1 {
                    Value::some(value).unwrap()
                } else {
                    Value::none()
                },
                increment + 1,
            )
        }
        TypeSignature::ResponseType(response) => {
            let (ok, increment_ok) = map_wasm_value(&response.0, index + 1, buffer);
            let (err, increment_err) =
                map_wasm_value(&response.1, index + 1 + increment_ok, buffer);
            (
                if buffer[index].unwrap_i32() == 1 {
                    Value::okay(ok).unwrap()
                } else {
                    Value::error(err).unwrap()
                },
                index + 1 + increment_ok + increment_err,
            )
        }
        // A `NoType` will be a dummy value that should not be used.
        TypeSignature::NoType => (Value::none(), 1),
        _ => panic!("WASM value type not implemented: {:?}", type_sig),
    }
}

#[test]
fn test_map_wasm_value() {
    let mut type_sig = TypeSignature::IntType;
    let mut buffer = vec![Val::I64(0x123), Val::I64(0x456)];
    let mut value = map_wasm_value(&type_sig, 0, &buffer);
    assert_eq!(value.0, Value::Int(0x123_0000_0000_0000_0456));
    assert_eq!(value.1, 2);

    type_sig = TypeSignature::UIntType;
    buffer = vec![Val::I64(-8690466096661279831), Val::I64(0x123456789abcdef0)];
    value = map_wasm_value(&type_sig, 0, &buffer);
    assert_eq!(
        value.0,
        Value::UInt(0x8765_4321_0fed_cba9_1234_5678_9abc_def0)
    );
    assert_eq!(value.1, 2);

    type_sig = TypeSignature::BoolType;
    buffer = vec![Val::I32(1)];
    value = map_wasm_value(&type_sig, 0, &buffer);
    assert_eq!(value.0, Value::Bool(true));
    assert_eq!(value.1, 1);

    type_sig = TypeSignature::BoolType;
    buffer = vec![Val::I32(0)];
    value = map_wasm_value(&type_sig, 0, &buffer);
    assert_eq!(value.0, Value::Bool(false));
    assert_eq!(value.1, 1);

    type_sig = TypeSignature::OptionalType(Box::new(TypeSignature::IntType));
    buffer = vec![Val::I32(1), Val::I64(0x123), Val::I64(0x456)];
    value = map_wasm_value(&type_sig, 0, &buffer);
    assert_eq!(
        value.0,
        Value::some(Value::Int(0x123_0000_0000_0000_0456)).unwrap()
    );
    assert_eq!(value.1, 3);

    type_sig = TypeSignature::OptionalType(Box::new(TypeSignature::IntType));
    buffer = vec![Val::I32(0), Val::I64(0x123), Val::I64(0x456)];
    value = map_wasm_value(&type_sig, 0, &buffer);
    assert_eq!(value.0, Value::none());
    assert_eq!(value.1, 3);

    type_sig =
        TypeSignature::ResponseType(Box::new((TypeSignature::IntType, TypeSignature::BoolType)));
    buffer = vec![Val::I32(1), Val::I64(0x123), Val::I64(0x456), Val::I32(0)];
    value = map_wasm_value(&type_sig, 0, &buffer);
    assert_eq!(
        value.0,
        Value::okay(Value::Int(0x123_0000_0000_0000_0456)).unwrap()
    );
    assert_eq!(value.1, 4);

    type_sig =
        TypeSignature::ResponseType(Box::new((TypeSignature::IntType, TypeSignature::BoolType)));
    buffer = vec![Val::I32(0), Val::I64(0x123), Val::I64(0x456), Val::I32(1)];
    value = map_wasm_value(&type_sig, 0, &buffer);
    assert_eq!(value.0, Value::error(Value::Bool(true)).unwrap());
    assert_eq!(value.1, 4);
}

impl<'a, 'b> WasmtimeHelper<'a, 'b> {
    pub fn new(
        contract_id: QualifiedContractIdentifier,
        global_context: &'b mut GlobalContext<'a>,
        contract_context: &'b mut ContractContext,
        compile_result: &mut CompileResult,
    ) -> Self {
        let contract_str =
            std::fs::read_to_string(format!("contracts/{}.clar", contract_id.name)).unwrap();

        let cost_tracker = LimitedCostTracker::Free;
        let clarity_store = MemoryBackingStore::new();

        let wasm = compile_result.module.emit_wasm();
        let contract_analysis = &compile_result.contract_analysis;
        let context =
            ClarityWasmContext::new(global_context, contract_context, contract_analysis.clone());

        let engine = Engine::default();
        let mut store = Store::new(&engine, context);
        let mut linker = Linker::new(&engine);

        linker
            .func_wrap(
                "clarity",
                "define_variable",
                |mut caller: Caller<'_, ClarityWasmContext>,
                 identifier: i32,
                 name_offset: i32,
                 name_length: i32,
                 value_offset: i32,
                 value_length: i32| {
                    // Read the variable name string from the memory
                    let name = WasmtimeHelper::read_identifier_from_wasm(
                        &mut caller,
                        name_offset,
                        name_length,
                    );

                    // Read the initial value from the memory
                    let ty = caller
                        .data()
                        .contract_analysis
                        .get_persisted_variable_type(name.as_str())
                        .expect("failed to get variable type")
                        .clone();
                    let contract = caller.data().contract_context.contract_identifier.clone();
                    let epoch = caller.data().global_context.epoch_id;
                    let value = WasmtimeHelper::read_from_wasm(
                        &mut caller,
                        &ty,
                        value_offset,
                        value_length,
                    );

                    // Store the mapping of variable name to identifier
                    caller
                        .data_mut()
                        .identifier_map
                        .insert(identifier, name.clone());

                    // Create the variable in the global context
                    let data_types = caller.data_mut().global_context.database.create_variable(
                        &contract,
                        name.as_str(),
                        ty,
                    );

                    // Store the variable in the global context
                    caller
                        .data_mut()
                        .global_context
                        .database
                        .set_variable(&contract, name.as_str(), value, &data_types)
                        .unwrap();

                    caller
                        .data_mut()
                        .contract_context
                        .meta_data_var
                        .insert(ClarityName::from(name.as_str()), data_types.clone());
                },
            )
            .unwrap();

        linker
            .func_wrap(
                "clarity",
                "get_variable",
                |mut caller: Caller<'_, ClarityWasmContext>,
                 identifier: i32,
                 return_offset: i32,
                 return_length: i32| {
                    let var_name = caller
                        .data()
                        .identifier_map
                        .get(&identifier)
                        .expect("failed to get variable name")
                        .clone();
                    let contract = caller.data().contract_context.contract_identifier.clone();
                    let data_types = caller
                        .data()
                        .contract_context
                        .meta_data_var
                        .get(var_name.as_str())
                        .unwrap()
                        .clone(); // FIXME
                    let value = caller
                        .data_mut()
                        .global_context
                        .database
                        .lookup_variable_with_size(&contract, var_name.as_str(), &data_types)
                        .unwrap()
                        .value;

                    WasmtimeHelper::write_to_wasm(
                        &mut caller,
                        &data_types.value_type,
                        return_offset,
                        return_length,
                        value,
                    );
                },
            )
            .unwrap();

        linker
            .func_wrap(
                "clarity",
                "set_variable",
                |mut caller: Caller<'_, ClarityWasmContext>,
                 identifier: i32,
                 value_offset: i32,
                 value_length: i32| {
                    let var_name = caller
                        .data()
                        .identifier_map
                        .get(&identifier)
                        .expect("failed to get variable name")
                        .clone();

                    let ty = caller
                        .data()
                        .contract_analysis
                        .get_persisted_variable_type(var_name.as_str())
                        .expect("failed to get variable type")
                        .clone();

                    // Read in the value from the Wasm memory
                    let value = WasmtimeHelper::read_from_wasm(
                        &mut caller,
                        &ty,
                        value_offset,
                        value_length,
                    );

                    let contract = caller.data().contract_context.contract_identifier.clone();
                    let epoch = caller.data().global_context.epoch_id;
                    let data_types = caller
                        .data()
                        .contract_context
                        .meta_data_var
                        .get(var_name.as_str())
                        .unwrap()
                        .clone(); // FIXME

                    // Store the variable in the global context
                    caller
                        .data_mut()
                        .global_context
                        .database
                        .set_variable(&contract, var_name.as_str(), value, &data_types)
                        .unwrap();
                },
            )
            .unwrap();

        // Create a log function for debugging.
        linker
            .func_wrap(
                "",
                "log",
                |_: Caller<'_, ClarityWasmContext>, param: i64| {
                    println!("log: {param}");
                },
            )
            .unwrap();

        let module = Module::from_binary(&engine, wasm.as_slice()).unwrap();
        let instance = linker.instantiate(store.as_context_mut(), &module).unwrap();

        WasmtimeHelper {
            instance,
            store: Box::new(store),
        }
    }

    /// Read an identifier (string) from the WASM memory at `offset` with `length`.
    fn read_identifier_from_wasm(
        caller: &mut Caller<'_, ClarityWasmContext>,
        offset: i32,
        length: i32,
    ) -> String {
        // Get the memory from the caller
        let memory = caller
            .get_export("memory")
            .and_then(|export| export.into_memory())
            .expect("instance memory export");

        let mut buffer: Vec<u8> = vec![0; length as usize];
        memory
            .read(caller, offset as usize, &mut buffer)
            .expect("failed to read variable name");
        String::from_utf8(buffer).expect("failed to convert memory contents to string")
    }

    /// Read a value from the WASM memory at `offset` with `length` given the provided
    /// Clarity `TypeSignature`.
    fn read_from_wasm(
        caller: &mut Caller<'_, ClarityWasmContext>,
        ty: &TypeSignature,
        offset: i32,
        length: i32,
    ) -> Value {
        // Get the memory from the caller
        let memory = caller
            .get_export("memory")
            .and_then(|export| export.into_memory())
            .expect("instance memory export");

        match ty {
            TypeSignature::UIntType => {
                assert!(
                    length == 16,
                    "expected uint length to be 16 bytes, found {}",
                    length
                );
                let mut buffer: [u8; 8] = [0; 8];
                memory
                    .read(caller.borrow_mut(), offset as usize, &mut buffer)
                    .expect("failed to read int");
                let high = u64::from_le_bytes(buffer) as u128;
                memory
                    .read(caller.borrow_mut(), (offset + 8) as usize, &mut buffer)
                    .expect("failed to read int");
                let low = u64::from_le_bytes(buffer) as u128;
                Value::UInt((high << 64) | low)
            }
            TypeSignature::IntType => {
                assert!(
                    length == 16,
                    "expected int length to be 16 bytes, found {}",
                    length
                );
                let mut buffer: [u8; 8] = [0; 8];
                memory
                    .read(caller.borrow_mut(), offset as usize, &mut buffer)
                    .expect("failed to read int");
                let high = u64::from_le_bytes(buffer) as u128;
                memory
                    .read(caller.borrow_mut(), (offset + 8) as usize, &mut buffer)
                    .expect("failed to read int");
                let low = u64::from_le_bytes(buffer) as u128;
                Value::Int(((high << 64) | low) as i128)
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                type_length,
            ))) => {
                assert!(
                    type_length
                        >= &BufferLength::try_from(length as u32).expect("invalid buffer length"),
                    "expected string length to be less than the type length"
                );
                let mut buffer: Vec<u8> = vec![0; length as usize];
                memory
                    .read(caller, offset as usize, &mut buffer)
                    .expect("failed to read variable name");
                Value::string_ascii_from_bytes(buffer)
                    .expect("failed to convert memory contents to string")
            }
            _ => panic!("unsupported type"),
        }
    }

    /// Write a value to the Wasm memory at `offset` with `length` given the
    /// provided Clarity `TypeSignature`.'
    fn write_to_wasm(
        caller: &mut Caller<'_, ClarityWasmContext>,
        ty: &TypeSignature,
        offset: i32,
        length: i32,
        value: Value,
    ) {
        let memory = caller
            .get_export("memory")
            .and_then(|export| export.into_memory())
            .expect("instance memory export");

        match ty {
            TypeSignature::IntType => {
                assert!(
                    length == 16,
                    "expected int length to be 16 bytes, found {}",
                    length
                );
                let mut buffer: [u8; 8] = [0; 8];
                let i = value.expect_i128();
                let high = (i >> 64) as u64;
                let low = (i & 0xffff_ffff_ffff_ffff) as u64;
                buffer.copy_from_slice(&high.to_le_bytes());
                memory
                    .write(caller.borrow_mut(), offset as usize, &buffer)
                    .expect("failed to write int");
                buffer.copy_from_slice(&low.to_le_bytes());
                memory
                    .write(caller.borrow_mut(), (offset + 8) as usize, &buffer)
                    .expect("failed to write int");
            }
            _ => panic!("unsupported type"),
        };
    }
    /// Calls the specified public Clarity function in the generated contract WASM binary.
    pub fn call_public_function(&mut self, name: &str, params: &[Val]) -> Result<Value, String> {
        let fn_type = self
            .store
            .data()
            .contract_analysis
            .get_public_function_type(name)
            .expect("Function not found")
            .clone();

        let func_type = generate_wasmtime_func_signature(&fn_type);

        let func = self
            .instance
            .get_func(self.store.as_context_mut(), name)
            .expect("Provided function name was not found in the generated WASM binary.");

        let mut results = vec![Val::I32(0); func_type.results().len()];

        match func.call(self.store.as_context_mut(), params, &mut results) {
            Ok(_) => Ok(map_wasm_result(&fn_type, &results)),
            Err(e) => {
                println!("error: {}", e);
                Err(e.to_string())
            }
        }
    }
}
