use clarity_repl::clarity::{
    docs::{make_api_reference, make_define_reference, make_keyword_reference, FunctionAPI},
    functions::{define::DefineFunctions, NativeFunctions},
    variables::NativeVariables,
    ClarityVersion,
};
use lazy_static::lazy_static;
use std::collections::HashMap;

fn code(code: &str) -> String {
    vec!["```clarity", code.trim(), "```"].join("\n")
}

lazy_static! {
    pub static ref API_REF: HashMap<String, (ClarityVersion, String, Option<FunctionAPI>)> = {
        let mut api_references: HashMap<String, (ClarityVersion, String, Option<FunctionAPI>)> = HashMap::new();
         // "---" can produce h2 if placed under text
        let separator = "- - -";

        for define_function in DefineFunctions::ALL {
            let reference = make_define_reference(define_function);
            api_references.insert(
                define_function.to_string(),
                (reference.version, Vec::from([
                    &code(&reference.signature),
                    separator,
                    "**Description**",
                    &reference.description,
                    separator,
                    "**Example**",
                    &code(&reference.example),
                ])
                .join("\n"), None),
            );
        }

        for native_function in NativeFunctions::ALL {
            let reference = make_api_reference(native_function);
            api_references.insert(
                native_function.to_string(),
                (reference.version, Vec::from([
                    &code(format!("{} -> {}", &reference.signature, &reference.output_type).as_str()),
                    separator,
                    "**Description**",
                    &reference.description,
                    separator,
                    "**Example**",
                    &code(&reference.example),
                    separator,
                    &format!("**Introduced in:** {}", &reference.version),
                ])
                .join("\n"), Some(reference)),
            );
        }

        for native_keyword in NativeVariables::ALL {
            let reference = make_keyword_reference(native_keyword).unwrap();
            api_references.insert(
                native_keyword.to_string(),
                (reference.version, Vec::from([
                    "**Description**",
                    &reference.description,
                    separator,
                    "**Example**",
                    &code(&reference.example),
                    separator,
                    &format!("**Introduced in:** {}", &reference.version),
                ])
                .join("\n"), None),
            );
        }

        api_references
    };
}
