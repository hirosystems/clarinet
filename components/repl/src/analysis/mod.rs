pub mod annotation;
pub mod ast_dependency_detector;
pub mod ast_visitor;
pub mod call_checker;
pub mod check_checker;

use serde::de::Deserialize;
use serde::Serialize;

use crate::analysis::annotation::Annotation;
use crate::clarity::analysis::analysis_db::AnalysisDatabase;
use crate::clarity::analysis::types::ContractAnalysis;
use crate::clarity::diagnostic::Diagnostic;

use self::ast_dependency_detector::ASTDependencyDetector;
use self::call_checker::CallChecker;
use self::check_checker::CheckChecker;

pub type AnalysisResult = Result<Vec<Diagnostic>, Vec<Diagnostic>>;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pass {
    All,
    CheckChecker,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Settings {
    passes: Vec<Pass>,
    check_checker: check_checker::Settings,
}

impl Settings {
    pub fn enable_all_passes(&mut self) {
        self.passes = ALL_PASSES.to_vec();
    }

    pub fn set_passes(&mut self, passes: Vec<Pass>) {
        for pass in passes {
            match pass {
                Pass::All => {
                    self.passes = ALL_PASSES.to_vec();
                    return;
                }
                pass => self.passes.push(pass),
            };
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OneOrList<T> {
    /// Allow `T` as shorthand for `[T]` in the TOML
    One(T),
    /// Allow more than one `T` in the TOML
    List(Vec<T>),
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SettingsFile {
    passes: Option<OneOrList<Pass>>,
    check_checker: Option<check_checker::SettingsFile>,
}

// Each new pass should be included in this list
static ALL_PASSES: [Pass; 1] = [Pass::CheckChecker];

impl From<SettingsFile> for Settings {
    fn from(from_file: SettingsFile) -> Self {
        let passes = if let Some(file_passes) = from_file.passes {
            match file_passes {
                OneOrList::One(pass) => match pass {
                    Pass::All => ALL_PASSES.to_vec(),
                    pass => vec![pass],
                },
                OneOrList::List(passes) => {
                    if passes.contains(&Pass::All) {
                        ALL_PASSES.to_vec()
                    } else {
                        passes
                    }
                }
            }
        } else {
            vec![]
        };

        // Each pass that has its own settings should be included here.
        let checker_settings = if let Some(checker_settings) = from_file.check_checker {
            check_checker::Settings::from(checker_settings)
        } else {
            check_checker::Settings::default()
        };

        Self {
            passes,
            check_checker: checker_settings,
        }
    }
}

pub trait AnalysisPass {
    fn run_pass(
        contract_analysis: &mut ContractAnalysis,
        analysis_db: &mut AnalysisDatabase,
        annotations: &Vec<Annotation>,
        settings: &Settings,
    ) -> AnalysisResult;
}

pub fn run_analysis(
    contract_analysis: &mut ContractAnalysis,
    analysis_db: &mut AnalysisDatabase,
    annotations: &Vec<Annotation>,
    settings: &Settings,
) -> AnalysisResult {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut passes: Vec<
        fn(
            &mut ContractAnalysis,
            &mut AnalysisDatabase,
            &Vec<Annotation>,
            settings: &Settings,
        ) -> AnalysisResult,
    > = vec![CallChecker::run_pass];
    for pass in &settings.passes {
        match pass {
            Pass::CheckChecker => passes.push(CheckChecker::run_pass),
            Pass::All => panic!("unexpected All in list of passes"),
        }
    }

    analysis_db.execute(|db| {
        for pass in passes {
            // Collect warnings and continue, or if there is an error, return.
            match pass(contract_analysis, db, annotations, &settings) {
                Ok(mut w) => errors.append(&mut w),
                Err(mut e) => {
                    errors.append(&mut e);
                    return Err(errors);
                }
            }
        }
        Ok(errors)
    })
}
