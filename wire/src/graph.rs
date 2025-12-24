use crate::models::ProviderInfo;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Node {
    pub provider: ProviderInfo,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub nodes: HashMap<String, Node>,
    pub edges: HashMap<String, Vec<String>>,
}

impl Graph {
    pub fn new(providers: &[ProviderInfo], wrappers: Vec<String>) -> std::result::Result<Self, String> {
        let mut graph = Graph::default();
        let mut type_to_providers: HashMap<String, Vec<String>> = HashMap::new();

        // Step 1: Check for duplicates
        for p in providers {
            let ty = normalize_type(&p.ret, &wrappers);
            type_to_providers.entry(ty).or_default().push(p.path.clone());
        }

        let mut conflict_errors = Vec::new();
        for (ty, paths) in type_to_providers.iter() {
            if paths.len() > 1 {
                conflict_errors.push(format!(
                    "Multiple providers found for type '{}': {:?}",
                    ty, paths
                ));
            }
        }

        if !conflict_errors.is_empty() {
            return Err(conflict_errors.join("\n"));
        }

        // Step 2: Build the graph
        for p in providers {
            let ty = normalize_type(&p.ret, &wrappers);
            let dependencies: Vec<String> =
                p.args.iter().map(|arg| normalize_type(&arg.ty, &wrappers)).collect();

            graph.nodes.insert(
                ty.clone(),
                Node {
                    provider: p.clone(),
                },
            );

            graph.edges.insert(ty, dependencies);
        }

        Ok(graph)
    }

    pub fn resolve(&self, target_ty: &str) -> std::result::Result<Vec<ProviderInfo>, String> {
        if self.nodes.is_empty() {
            return Err("No providers found.".to_string());
        }

        if !self.nodes.contains_key(target_ty) {
            let available: Vec<_> = self.nodes.keys().cloned().collect();
            return Err(format!(
                "Missing provider for type: {}. Available types: {:?}",
                target_ty, available
            ));
        }

        let mut sorted_providers = Vec::new();
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();

        self.visit(
            target_ty,
            &mut visiting,
            &mut visited,
            &mut sorted_providers,
        )?;

        Ok(sorted_providers)
    }

    fn visit(
        &self,
        ty: &str,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        sorted_providers: &mut Vec<ProviderInfo>,
    ) -> std::result::Result<(), String> {
        if visited.contains(ty) {
            return Ok(());
        }
        if visiting.contains(ty) {
            return Err(format!("Circular dependency detected on type: {}", ty));
        }
        let node = self
            .nodes
            .get(ty)
            .ok_or_else(|| format!("Missing provider for type: {}", ty))?;

        visiting.insert(ty.to_string());
        if let Some(dependencies) = self.edges.get(ty) {
            for dep in dependencies {
                self.visit(dep, visiting, visited, sorted_providers)?;
            }
        }
        visiting.remove(ty);
        visited.insert(ty.to_string());
        sorted_providers.push(node.provider.clone());

        Ok(())
    }
}

pub(crate) fn normalize_type(ty_str: &str, wrappers: &[String]) -> String {
    let mut s = ty_str.replace(" ", "")
                 .replace("&", "")
                 .replace("'", "");
    
    // Recursive stripping of known wrappers
    loop {
        let mut changed = false;
        for w in wrappers {
            let prefix = format!("{}<", w);
            if s.starts_with(&prefix) && s.ends_with(">") {
                s = s[prefix.len()..s.len()-1].to_string();
                changed = true;
                break;
            }
        }
        if !changed { break; }
    }

    s.replace("<", "_")
     .replace(">", "_")
     .replace("(", "_")
     .replace(")", "_")
     .replace(",", "_")
     .to_lowercase()
}
