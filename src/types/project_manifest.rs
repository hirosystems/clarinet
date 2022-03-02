use clarity_repl::repl;
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::iter::FromIterator;
use std::path::PathBuf;
use std::process;
use toml::value::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectManifestFile {
    project: ProjectConfigFile,
    contracts: Option<Value>,
    repl: Option<repl::SettingsFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfigFile {
    name: String,
    authors: Option<Vec<String>>,
    description: Option<String>,
    telemetry: Option<bool>,
    requirements: Option<Value>,

    // The fields below have been moved into repl above, but are kept here for
    // backwards compatibility.
    analysis: Option<Vec<clarity_repl::analysis::Pass>>,
    costs_version: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ProjectManifest {
    pub project: ProjectConfig,
    #[serde(serialize_with = "toml::ser::tables_last")]
    pub contracts: BTreeMap<String, ContractConfig>,
    #[serde(rename = "repl")]
    pub repl_settings: repl::Settings,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ProjectConfig {
    pub name: String,
    pub authors: Vec<String>,
    pub description: String,
    pub telemetry: bool,
    pub requirements: Option<Vec<RequirementConfig>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RequirementConfig {
    pub contract_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractConfig {
    pub path: String,
    pub depends_on: Vec<String>,
    pub deployer: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotebookConfig {
    pub name: String,
    pub path: String,
}

impl ProjectManifest {
    pub fn from_path(path: &PathBuf) -> ProjectManifest {
        let path = match File::open(path) {
            Ok(path) => path,
            Err(_e) => {
                println!("Error: unable to locate Clarinet.toml in current directory");
                std::process::exit(1);
            }
        };
        let mut project_manifest_file_reader = BufReader::new(path);
        let mut project_manifest_file_buffer = vec![];
        project_manifest_file_reader
            .read_to_end(&mut project_manifest_file_buffer)
            .unwrap();

        let project_manifest_file: ProjectManifestFile =
            match toml::from_slice(&project_manifest_file_buffer[..]) {
                Ok(s) => s,
                Err(_e) => {
                    println!(
                        "{}\n{:?}",
                        red!("Error: there is an issue with the Clarinet.toml file"),
                        _e
                    );
                    std::process::exit(1);
                }
            };

        ProjectManifest::from_project_manifest_file(project_manifest_file)
    }

    pub fn ordered_contracts(&self) -> Vec<(String, ContractConfig)> {
        let mut dst = vec![];
        let mut lookup = BTreeMap::new();
        let mut reverse_lookup = BTreeMap::new();

        let mut index: usize = 0;
        let contracts = self.contracts.clone();

        if contracts.is_empty() {
            return vec![];
        }

        for (contract, _) in contracts.iter() {
            lookup.insert(contract, index);
            reverse_lookup.insert(index, contract.clone());
            index += 1;
        }

        let mut graph = Graph::new();
        for (contract, contract_config) in contracts.iter() {
            let contract_id = lookup.get(contract).unwrap();
            graph.add_node(*contract_id);
            for deps in contract_config.depends_on.iter() {
                let dep_id = lookup.get(deps).unwrap();
                graph.add_directed_edge(*contract_id, *dep_id);
            }
        }

        let mut walker = GraphWalker::new();
        let sorted_indexes = walker.get_sorted_dependencies(&graph);

        let cyclic_deps = walker.get_cycling_dependencies(&graph, &sorted_indexes);
        if let Some(deps) = cyclic_deps {
            let mut contracts = vec![];
            for index in deps.iter() {
                let contract = {
                    let entry = reverse_lookup.get(index).unwrap();
                    entry.clone()
                };
                contracts.push(contract);
            }
            println!("Error: cycling dependencies: {}", contracts.join(", "));
            process::exit(0);
        }

        for index in sorted_indexes.iter() {
            let contract = {
                let entry = reverse_lookup.get(index).unwrap();
                entry.clone()
            };
            let config = contracts.get(&contract).unwrap();
            dst.push((contract, config.clone()))
        }
        dst
    }

    pub fn from_project_manifest_file(
        project_manifest_file: ProjectManifestFile,
    ) -> ProjectManifest {
        let project = ProjectConfig {
            name: project_manifest_file.project.name.clone(),
            requirements: None,
            description: project_manifest_file
                .project
                .description
                .unwrap_or("".into()),
            authors: project_manifest_file.project.authors.unwrap_or(vec![]),
            telemetry: project_manifest_file.project.telemetry.unwrap_or(false),
        };

        let mut repl_settings = if let Some(repl_settings) = project_manifest_file.repl {
            repl::Settings::from(repl_settings)
        } else {
            repl::Settings::default()
        };

        // Check for deprecated settings
        if let Some(passes) = project_manifest_file.project.analysis {
            println!(
                "{}: use of 'project.analysis' in Clarinet.toml is deprecated; use repl.analysis.passes",
                yellow!("warning")
            );
            repl_settings.analysis.set_passes(passes);
        }
        if let Some(costs_version) = project_manifest_file.project.costs_version {
            println!(
                "{}: use of 'project.costs_version' in Clarinet.toml is deprecated; use repl.costs_version",
                yellow!("warning")
            );
            repl_settings.costs_version = costs_version;
        }

        let mut config = ProjectManifest {
            project,
            contracts: BTreeMap::new(),
            repl_settings,
        };
        let mut config_contracts = BTreeMap::new();
        let mut config_requirements: Vec<RequirementConfig> = Vec::new();

        match project_manifest_file.project.requirements {
            Some(Value::Array(requirements)) => {
                for link_settings in requirements.iter() {
                    match link_settings {
                        Value::Table(link_settings) => {
                            let contract_id = match link_settings.get("contract_id") {
                                Some(Value::String(contract_id)) => contract_id.to_string(),
                                _ => continue,
                            };
                            config_requirements.push(RequirementConfig { contract_id });
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };

        match project_manifest_file.contracts {
            Some(Value::Table(contracts)) => {
                for (contract_name, contract_settings) in contracts.iter() {
                    match contract_settings {
                        Value::Table(contract_settings) => {
                            let path = match contract_settings.get("path") {
                                Some(Value::String(path)) => path.to_string(),
                                _ => continue,
                            };
                            let depends_on = match contract_settings.get("depends_on") {
                                Some(Value::Array(depends_on)) => depends_on
                                    .iter()
                                    .map(|v| v.as_str().unwrap().to_string())
                                    .collect::<Vec<String>>(),
                                _ => continue,
                            };
                            let deployer = match contract_settings.get("deployer") {
                                Some(Value::String(deployer)) => deployer,
                                _ => "deployer",
                            };
                            config_contracts.insert(
                                contract_name.to_string(),
                                ContractConfig {
                                    path,
                                    depends_on,
                                    deployer: deployer.to_string(),
                                },
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };
        config.contracts = config_contracts;
        config.project.requirements = Some(config_requirements);
        config
    }
}

struct Graph {
    pub adjacency_list: Vec<Vec<usize>>,
}

impl Graph {
    fn new() -> Self {
        Self {
            adjacency_list: Vec::new(),
        }
    }

    fn add_node(&mut self, _expr_index: usize) {
        self.adjacency_list.push(vec![]);
    }

    fn add_directed_edge(&mut self, src_expr_index: usize, dst_expr_index: usize) {
        let list = self.adjacency_list.get_mut(src_expr_index).unwrap();
        list.push(dst_expr_index);
    }

    fn get_node_descendants(&self, expr_index: usize) -> Vec<usize> {
        self.adjacency_list[expr_index].clone()
    }

    fn has_node_descendants(&self, expr_index: usize) -> bool {
        self.adjacency_list[expr_index].len() > 0
    }

    fn nodes_count(&self) -> usize {
        self.adjacency_list.len()
    }
}

struct GraphWalker {
    seen: HashSet<usize>,
}

impl GraphWalker {
    fn new() -> Self {
        Self {
            seen: HashSet::new(),
        }
    }

    /// Depth-first search producing a post-order sort
    fn get_sorted_dependencies(&mut self, graph: &Graph) -> Vec<usize> {
        let mut sorted_indexes = Vec::<usize>::new();
        for expr_index in 0..graph.nodes_count() {
            self.sort_dependencies_recursion(expr_index, graph, &mut sorted_indexes);
        }

        sorted_indexes
    }

    fn sort_dependencies_recursion(
        &mut self,
        tle_index: usize,
        graph: &Graph,
        branch: &mut Vec<usize>,
    ) {
        if self.seen.contains(&tle_index) {
            return;
        }

        self.seen.insert(tle_index);
        if let Some(list) = graph.adjacency_list.get(tle_index) {
            for neighbor in list.iter() {
                self.sort_dependencies_recursion(neighbor.clone(), graph, branch);
            }
        }
        branch.push(tle_index);
    }

    fn get_cycling_dependencies(
        &mut self,
        graph: &Graph,
        sorted_indexes: &Vec<usize>,
    ) -> Option<Vec<usize>> {
        let mut tainted: HashSet<usize> = HashSet::new();

        for node in sorted_indexes.iter() {
            let mut tainted_descendants_count = 0;
            let descendants = graph.get_node_descendants(*node);
            for descendant in descendants.iter() {
                if !graph.has_node_descendants(*descendant) || tainted.contains(descendant) {
                    tainted.insert(*descendant);
                    tainted_descendants_count += 1;
                }
            }
            if tainted_descendants_count == descendants.len() {
                tainted.insert(*node);
            }
        }

        if tainted.len() == sorted_indexes.len() {
            return None;
        }

        let nodes = HashSet::from_iter(sorted_indexes.iter().cloned());
        let deps = nodes.difference(&tainted).map(|i| *i).collect();
        Some(deps)
    }
}
