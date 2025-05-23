use crate::DevnetConfig;

/// Config which fields to check for differences
pub struct DevnetDiffConfig {
    /// Fields to check for differences
    significant_fields: Vec<SignificantField>,
}

/// Represents a field that should be checked for differences
pub struct SignificantField {
    /// Name of the field for display purposes
    pub name: String,
    /// Function to extract the value from a DevnetConfig
    pub extractor: Extractor,
}
type Extractor = Box<dyn Fn(&DevnetConfig) -> String>;

/// Creates an extractor for any field that implements ToString
fn make_extractor<T: ToString + 'static>(accessor: fn(&DevnetConfig) -> T) -> Extractor {
    Box::new(move |config| accessor(config).to_string())
}

impl DevnetDiffConfig {
    /// Create a new diff configuration with default significant fields
    pub fn new() -> Self {
        Self {
            significant_fields: Self::default_significant_fields(),
        }
    }

    /// Create a diff configuration with custom fields
    pub fn with_fields(fields: Vec<(&'static str, Extractor)>) -> Self {
        Self {
            significant_fields: fields
                .into_iter()
                .map(|(name, extractor)| SignificantField {
                    name: name.to_string(),
                    extractor,
                })
                .collect(),
        }
    }

    /// Get the default set of significant fields
    fn default_significant_fields() -> Vec<SignificantField> {
        vec![
            // Epoch configurations
            SignificantField {
                name: "epoch_2_0".to_string(),
                extractor: make_extractor(|config| config.epoch_2_0),
            },
            SignificantField {
                name: "epoch_2_05".to_string(),
                extractor: make_extractor(|config| config.epoch_2_05),
            },
            SignificantField {
                name: "epoch_2_1".to_string(),
                extractor: make_extractor(|config| config.epoch_2_1),
            },
            SignificantField {
                name: "epoch_2_2".to_string(),
                extractor: make_extractor(|config| config.epoch_2_2),
            },
            SignificantField {
                name: "epoch_2_3".to_string(),
                extractor: make_extractor(|config| config.epoch_2_3),
            },
            SignificantField {
                name: "epoch_2_4".to_string(),
                extractor: make_extractor(|config| config.epoch_2_4),
            },
            SignificantField {
                name: "epoch_2_5".to_string(),
                extractor: make_extractor(|config| config.epoch_2_5),
            },
            SignificantField {
                name: "epoch_3_0".to_string(),
                extractor: make_extractor(|config| config.epoch_3_0),
            },
            SignificantField {
                name: "epoch_3_1".to_string(),
                extractor: make_extractor(|config| config.epoch_3_1),
            },
            // Container configuration
            SignificantField {
                name: "bind_containers_volumes".to_string(),
                extractor: make_extractor(|config| config.bind_containers_volumes),
            },
            // Image URLs
            SignificantField {
                name: "bitcoin_node_image_url".to_string(),
                extractor: make_extractor(|config| config.bitcoin_node_image_url.clone()),
            },
            SignificantField {
                name: "stacks_node_image_url".to_string(),
                extractor: make_extractor(|config| config.stacks_node_image_url.clone()),
            },
            SignificantField {
                name: "stacks_api_image_url".to_string(),
                extractor: make_extractor(|config| config.stacks_api_image_url.clone()),
            },
        ]
    }

    /// Compare two DevnetConfig instances and return fields that are different
    pub fn get_different_fields(
        &self,
        default_config: &DevnetConfig,
        user_config: &DevnetConfig,
    ) -> Vec<String> {
        let mut different_fields = Vec::new();

        for field in &self.significant_fields {
            let default_value = (field.extractor)(default_config);
            let user_value = (field.extractor)(user_config);

            if default_value != user_value {
                different_fields.push(field.name.clone())
            }
        }

        different_fields
    }

    /// Check if any significant fields are different
    pub fn has_differences(
        &self,
        default_config: &DevnetConfig,
        user_config: &DevnetConfig,
    ) -> bool {
        for field in &self.significant_fields {
            let default_value = (field.extractor)(default_config);
            let user_value = (field.extractor)(user_config);

            if default_value != user_value {
                return true;
            }
        }
        false
    }

    /// Get the names of fields that are different
    pub fn get_different_field_names(
        &self,
        default_config: &DevnetConfig,
        user_config: &DevnetConfig,
    ) -> Vec<String> {
        self.get_different_fields(default_config, user_config)
            .into_iter()
            .collect()
    }

    /// Generate a simple report of different fields
    pub fn generate_report(
        &self,
        default_config: &DevnetConfig,
        user_config: &DevnetConfig,
    ) -> String {
        let different_fields = self.get_different_fields(default_config, user_config);

        if different_fields.is_empty() {
            return "No significant differences found between user and default configuration."
                .to_string();
        }

        let mut report = format!(
            "Found {} significant difference(s):\n\n",
            different_fields.len()
        );

        for field in different_fields {
            report.push_str(&format!("• {}\n", field));

            report.push('\n');
        }

        report
    }
}

pub fn default_significant_fields() -> Vec<(&'static str, Extractor)> {
    vec![
        ("epoch_2_0", make_extractor(|c| c.epoch_2_0)),
        ("epoch_2_05", make_extractor(|c| c.epoch_2_05)),
        ("epoch_2_1", make_extractor(|c| c.epoch_2_1)),
        ("epoch_2_2", make_extractor(|c| c.epoch_2_2)),
        ("epoch_2_3", make_extractor(|c| c.epoch_2_3)),
        ("epoch_2_4", make_extractor(|c| c.epoch_2_4)),
        ("epoch_2_5", make_extractor(|c| c.epoch_2_5)),
        ("epoch_3_0", make_extractor(|c| c.epoch_3_0)),
        ("epoch_3_1", make_extractor(|c| c.epoch_3_1)),
    ]
}

impl Default for DevnetDiffConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_differences() {
        let config1 = DevnetConfig::default();
        let config2 = config1.clone();

        let differ = DevnetDiffConfig::new();
        assert!(!differ.has_differences(&config1, &config2));
        assert!(differ.get_different_fields(&config1, &config2).is_empty());
    }

    #[test]
    fn test_with_differences() {
        let default_config = DevnetConfig::default();
        let mut user_config = default_config.clone();
        user_config.epoch_3_0 = 150; // Different from default

        let differ = DevnetDiffConfig::new();
        assert!(differ.has_differences(&default_config, &user_config));

        let different_fields = differ.get_different_fields(&default_config, &user_config);
        assert!(different_fields.iter().any(|f| f == "epoch_3_0"));
    }

    #[test]
    fn test_custom_fields() {
        let custom_fields = vec![("epoch_3_1", make_extractor(|c| c.epoch_3_1.to_string()))];

        let differ = DevnetDiffConfig::with_fields(custom_fields);

        let default_config = DevnetConfig::default();
        let mut user_config = default_config.clone();
        user_config.epoch_3_1 = 146;

        let different_fields = differ.get_different_fields(&default_config, &user_config);
        assert_eq!(different_fields.len(), 1);
        assert_eq!(different_fields[0], "epoch_3_1");
    }
}
