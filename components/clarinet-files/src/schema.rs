use serde_json::{json, Value};

/// Generates JSON Schema for Clarinet.toml manifest file
pub fn generate_clarinet_manifest_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Clarinet Project Manifest",
        "description": "Configuration schema for Clarinet.toml files used in Clarity smart contract projects",
        "type": "object",
        "required": ["project"],
        "properties": {
            "project": {
                "type": "object",
                "description": "Project-level configuration",
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the project"
                    },
                    "authors": {
                        "type": "array",
                        "description": "List of project authors",
                        "items": {
                            "type": "string"
                        }
                    },
                    "description": {
                        "type": "string",
                        "description": "Project description"
                    },
                    "telemetry": {
                        "type": "boolean",
                        "description": "Enable or disable telemetry",
                        "default": false
                    },
                    "cache_dir": {
                        "type": "string",
                        "description": "Directory for caching build artifacts",
                        "default": ".cache"
                    },
                    "requirements": {
                        "type": "array",
                        "description": "External contract dependencies",
                        "items": {
                            "type": "object",
                            "required": ["contract_id"],
                            "properties": {
                                "contract_id": {
                                    "type": "string",
                                    "description": "Fully qualified contract identifier (e.g., SP2PABAF9FTAJYNFZH93XENAJ8FVY99RRM50D2JG9.nft-trait)"
                                }
                            }
                        }
                    },
                    "boot_contracts": {
                        "type": "array",
                        "description": "List of boot contracts to include (deprecated, kept for backwards compatibility)",
                        "items": {
                            "type": "string"
                        }
                    },
                    "override_boot_contracts_source": {
                        "type": "object",
                        "description": "Override default boot contract implementations with custom ones",
                        "additionalProperties": {
                            "type": "string",
                            "description": "Path to custom boot contract implementation"
                        },
                        "examples": [{
                            "pox-4": "./custom-boot-contracts/pox-4.clar",
                            "costs": "./custom-boot-contracts/costs.clar"
                        }]
                    },
                    "analysis": {
                        "type": "array",
                        "description": "Analysis passes to run (deprecated, use repl.analysis instead)",
                        "items": {
                            "type": "string"
                        }
                    }
                }
            },
            "contracts": {
                "type": "object",
                "description": "Contract definitions for the project",
                "additionalProperties": {
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the contract file from project root"
                        },
                        "deployer": {
                            "type": "string",
                            "description": "Deployer identifier (wallet name from settings/Devnet.toml)"
                        },
                        "clarity_version": {
                            "type": "integer",
                            "description": "Clarity language version to use",
                            "enum": [1, 2, 3],
                            "default": 1
                        },
                        "epoch": {
                            "description": "Stacks blockchain epoch",
                            "oneOf": [
                                {
                                    "type": "string",
                                    "enum": ["2.0", "2.05", "2.1", "2.2", "2.3", "2.4", "2.5", "3.0", "3.1", "3.2", "latest"]
                                },
                                {
                                    "type": "number",
                                    "enum": [2.0, 2.05, 2.1, 2.2, 2.3, 2.4, 2.5, 3.0, 3.1, 3.2]
                                }
                            ],
                            "default": "2.05"
                        }
                    }
                },
                "examples": [{
                    "counter": {
                        "path": "contracts/counter.clar",
                        "clarity_version": 2,
                        "epoch": "2.1"
                    }
                }]
            },
            "repl": {
                "type": "object",
                "description": "REPL and analysis settings",
                "properties": {
                    "analysis": {
                        "type": "object",
                        "description": "Static analysis configuration",
                        "properties": {
                            "passes": {
                                "description": "List of analysis passes to run (can be a single string or array)",
                                "oneOf": [
                                    {
                                        "type": "string",
                                        "enum": ["check_checker", "check_traits", "check_private_function_args"]
                                    },
                                    {
                                        "type": "array",
                                        "items": {
                                            "type": "string",
                                            "enum": ["check_checker", "check_traits", "check_private_function_args"]
                                        }
                                    }
                                ]
                            },
                            "check_checker": {
                                "type": "object",
                                "description": "Configuration for the check_checker analysis pass",
                                "properties": {
                                    "strict": {
                                        "type": "boolean",
                                        "description": "Strict mode sets all other options to false"
                                    },
                                    "trusted_sender": {
                                        "type": "boolean",
                                        "description": "After a filter on tx-sender, trust all inputs"
                                    },
                                    "trusted_caller": {
                                        "type": "boolean",
                                        "description": "After a filter on contract-caller, trust all inputs"
                                    },
                                    "callee_filter": {
                                        "type": "boolean",
                                        "description": "Allow filters in callee to filter caller"
                                    }
                                }
                            }
                        }
                    },
                    "remote_data": {
                        "type": "object",
                        "description": "Remote data fetching configuration",
                        "properties": {
                            "enabled": {
                                "type": "boolean",
                                "description": "Enable fetching data from remote sources",
                                "default": false
                            },
                            "api_url": {
                                "type": "string",
                                "description": "API URL for remote data fetching",
                                "format": "uri"
                            },
                            "initial_height": {
                                "type": "integer",
                                "description": "Initial blockchain height for data fetching",
                                "minimum": 0
                            },
                            "use_mainnet_wallets": {
                                "type": "boolean",
                                "description": "Use mainnet wallet addresses for testing",
                                "default": false
                            }
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_schema_is_valid_json() {
        let schema = generate_clarinet_manifest_schema();
        assert!(schema.is_object());
    }

    #[test]
    fn test_schema_has_required_fields() {
        let schema = generate_clarinet_manifest_schema();
        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(schema["title"], "Clarinet Project Manifest");
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["project"].is_object());
        assert!(schema["properties"]["contracts"].is_object());
        assert!(schema["properties"]["repl"].is_object());
    }

    #[test]
    fn test_contract_properties() {
        let schema = generate_clarinet_manifest_schema();
        let contract_schema = &schema["properties"]["contracts"]["additionalProperties"];

        assert_eq!(contract_schema["required"], json!(["path"]));
        assert!(contract_schema["properties"]["path"].is_object());
        assert!(contract_schema["properties"]["clarity_version"].is_object());
        assert!(contract_schema["properties"]["epoch"].is_object());
    }

    #[test]
    fn test_project_required_name() {
        let schema = generate_clarinet_manifest_schema();
        assert_eq!(schema["properties"]["project"]["required"], json!(["name"]));
    }

    #[test]
    fn test_clarity_version_enum() {
        let schema = generate_clarinet_manifest_schema();
        let clarity_version = &schema["properties"]["contracts"]["additionalProperties"]
            ["properties"]["clarity_version"];
        assert_eq!(clarity_version["enum"], json!([1, 2, 3]));
    }

    /// This test generates the schema file that editors can use
    /// Run with: cargo test --package clarinet-files generate_schema_file -- --ignored --nocapture
    #[test]
    #[ignore]
    fn generate_schema_file() {
        let schema = generate_clarinet_manifest_schema();
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();

        // Get the path to the schemas directory
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let mut schema_path = PathBuf::from(manifest_dir);
        schema_path.push("schemas");
        schema_path.push("clarinet-manifest.schema.json");

        // Write the schema file
        fs::write(&schema_path, schema_json).expect("Failed to write schema file");

        println!(" Schema file generated at: {}", schema_path.display());
    }

    /// This test verifies that the committed schema file is up-to-date
    #[test]
    fn test_schema_file_is_up_to_date() {
        let schema = generate_clarinet_manifest_schema();
        let generated_schema = serde_json::to_string_pretty(&schema).unwrap();

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let mut schema_path = PathBuf::from(manifest_dir);
        schema_path.push("schemas");
        schema_path.push("clarinet-manifest.schema.json");

        // Check if schema file exists
        if !schema_path.exists() {
            panic!(
                "Schema file does not exist at: {}\nRun: cargo test --package clarinet-files generate_schema_file -- --ignored",
                schema_path.display()
            );
        }

        // Read the existing schema file
        let existing_schema = fs::read_to_string(&schema_path).expect("Failed to read schema file");

        // Compare generated vs existing
        assert_eq!(
            generated_schema,
            existing_schema,
            "Schema file is out of date! Run: cargo test --package clarinet-files generate_schema_file -- --ignored"
        );
    }
}
