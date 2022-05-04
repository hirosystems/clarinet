pub mod mnemonic;
pub mod stacks;

use std::collections::{BTreeMap, HashSet};
use std::iter::FromIterator;
use std::process;
use std::future::Future;
use tokio;

pub fn create_basic_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .max_blocking_threads(32)
        .build()
        .unwrap()
}

pub fn nestable_block_on<F: Future>(future: F) -> F::Output {
    let (handle, _rt) = match tokio::runtime::Handle::try_current() {
        Ok(h) => (h, None),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            (rt.handle().clone(), Some(rt))
        }
    };
    let response = handle.block_on(async { future.await });
    response
}

pub fn order_contracts(src: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    if src.is_empty() {
        return vec![];
    }

    let mut dst = vec![];
    let mut lookup = BTreeMap::new();
    let mut reverse_lookup = BTreeMap::new();

    let mut index: usize = 0;

    for (contract_id, _) in src.iter() {
        lookup.insert(contract_id, index);
        reverse_lookup.insert(index, contract_id.clone());
        index += 1;
    }

    let mut graph = Graph::new();
    for (contract, dependencies) in src.iter() {
        let contract_id = lookup.get(contract).unwrap();
        graph.add_node(*contract_id);
        for deps in dependencies.iter() {
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
        dst.push(contract);
    }
    dst
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
