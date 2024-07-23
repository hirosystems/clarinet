use std::collections::HashMap;

use clarity::vm::{
    docs::{make_api_reference, make_define_reference, make_keyword_reference, FunctionAPI},
    functions::{define::DefineFunctions, NativeFunctions},
    variables::NativeVariables,
    ClarityVersion,
};

fn code(code: &str) -> String {
    ["```clarity", code.trim(), "```"].join("\n")
}

fn format_removed_in(max_version: Option<ClarityVersion>) -> String {
    max_version
        .map(|max_version| {
            format!(
                "Removed in **{}**",
                match max_version {
                    ClarityVersion::Clarity1 => ClarityVersion::Clarity2,
                    ClarityVersion::Clarity2 => ClarityVersion::Clarity3,
                    ClarityVersion::Clarity3 => ClarityVersion::latest(),
                }
            )
        })
        .unwrap_or_default()
}

lazy_static! {
    pub static ref API_REF: HashMap<String, (String, Option<FunctionAPI>)> = {
        let mut api_references: HashMap<String, (String, Option<FunctionAPI>)> = HashMap::new();
        let separator = "- - -";

        for define_function in DefineFunctions::ALL {
            let reference = make_define_reference(define_function);
            api_references.insert(
                define_function.to_string(),
                (
                    Vec::from([
                        &code(&reference.signature),
                        separator,
                        &reference.description,
                        separator,
                        "**Example**",
                        &code(&reference.example),
                    ])
                    .join("\n"),
                    Some(reference),
                ),
            );
        }

        for native_function in NativeFunctions::ALL {
            let reference = make_api_reference(native_function);
            api_references.insert(
                native_function.to_string(),
                (
                    Vec::from([
                        &code(
                            format!("{} -> {}", &reference.signature, &reference.output_type)
                                .as_str(),
                        ),
                        separator,
                        &reference.description,
                        separator,
                        &format!("Introduced in **{}**  ", &reference.min_version),
                        &format_removed_in(reference.max_version),
                        separator,
                        "**Example**",
                        &code(&reference.example),
                    ])
                    .join("\n"),
                    Some(reference),
                ),
            );
        }

        for native_keyword in NativeVariables::ALL {
            let reference = make_keyword_reference(native_keyword).unwrap();
            api_references.insert(
                native_keyword.to_string(),
                (
                    Vec::from([
                        reference.description,
                        separator,
                        &format!("Introduced in **{}**  ", &reference.min_version),
                        &format_removed_in(reference.max_version),
                        separator,
                        "**Example**",
                        &code(reference.example),
                    ])
                    .join("\n"),
                    None,
                ),
            );
        }

        api_references
    };
}
