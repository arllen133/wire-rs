use crate::parser::signature::ProviderSignature;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

pub struct DependencyGraph;

impl DependencyGraph {
    pub fn solve(providers: Vec<ProviderSignature>) -> Result<Vec<ProviderSignature>, String> {
        let mut graph = DiGraph::<ProviderSignature, ()>::new();
        let mut type_to_node = HashMap::new();

        for p in &providers {
            let idx = graph.add_node(p.clone());
            type_to_node.insert(p.output_type.clone(), idx);
        }

        for (idx, p) in providers.iter().enumerate() {
            let current_node = petgraph::graph::NodeIndex::new(idx);
            for input_type in &p.stripped_inputs {
                if let Some(&dep_node) = type_to_node.get(input_type) {
                    graph.add_edge(dep_node, current_node, ());
                }
            }
        }

        let sorted_indices = match toposort(&graph, None) {
            Ok(indices) => indices,
            Err(_) => {
                // Cycle detected! Find the cycle component.
                let sccs = petgraph::algo::tarjan_scc(&graph);
                let mut cycle_error = "Cycle detected in dependency graph!".to_string();
                
                for scc in sccs {
                    if scc.len() > 1 {
                        cycle_error.push_str("\n\nCycle involving:");
                        for node_idx in &scc {
                            let p = &graph[*node_idx];
                            cycle_error.push_str(&format!("\n  - {} (produces {})", p.name, p.output_type));
                        }
                    } else if scc.len() == 1 {
                         // Check for self-loop
                         let node_idx = scc[0];
                         if graph.contains_edge(node_idx, node_idx) {
                             let p = &graph[node_idx];
                             cycle_error.push_str(&format!("\n\nSelf-loop cycle: {} (produces {})", p.name, p.output_type));
                         }
                    }
                }
                return Err(cycle_error);
            }
        };

        Ok(sorted_indices
            .into_iter()
            .map(|i| graph[i].clone())
            .collect())
    }
}
