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

pub(crate) fn is_match(full: &str, suffix: &str) -> bool {
    full == suffix || full.ends_with(&format!("_{}", suffix))
}

impl Graph {
    pub fn new(providers: &[ProviderInfo], wrappers: Vec<String>) -> std::result::Result<Self, String> {
        let mut graph = Graph::default();
        let mut type_to_providers: HashMap<String, Vec<String>> = HashMap::new();

        // Step 1: Check for duplicates
        for p in providers {
            let ty = normalize_type(&p.ret, &wrappers);
            type_to_providers.entry(ty).or_default().push(p.path.clone());
            for b in &p.bindings {
                let ty_b = normalize_type(b, &wrappers);
                type_to_providers.entry(ty_b).or_default().push(p.path.clone());
            }
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
                p.args.iter().map(|arg| {
                    let lookup_ty = arg.from.as_ref().unwrap_or(&arg.ty);
                    normalize_type(lookup_ty, &wrappers)
                }).collect();

            graph.nodes.insert(
                ty.clone(),
                Node {
                    provider: p.clone(),
                },
            );
            graph.edges.insert(ty, dependencies.clone());

            for b in &p.bindings {
                let ty_b = normalize_type(b, &wrappers);
                graph.nodes.insert(
                    ty_b.clone(),
                    Node {
                        provider: p.clone(),
                    },
                );
                graph.edges.insert(ty_b, dependencies.clone());
            }
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
        let node = if let Some(n) = self.nodes.get(ty) {
            n
        } else {
            // Fuzzy matching
            let matched_key = self.nodes.keys()
                .find(|k| is_match(ty, k) || is_match(k, ty))
                .ok_or_else(|| {
                    let available: Vec<_> = self.nodes.keys().cloned().collect();
                    format!("Missing provider for type: {}. Available types: {:?}", ty, available)
                })?;
            self.nodes.get(matched_key).unwrap()
        };

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
            let search = format!("{}<", w);
            if let Some(pos) = s.find(&search) {
                if s.ends_with('>') {
                    let prefix = &s[..pos];
                    if prefix.is_empty() || prefix.ends_with("::") {
                        s = s[pos + search.len()..s.len() - 1].to_string();
                        changed = true;
                        break;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    s = s.replace("<", "_")
         .replace(">", "_")
         .replace("(", "_")
         .replace(")", "_")
         .replace(",", "_")
         .replace("::", "_")
         .to_lowercase();

    if s.starts_with("dyn") {
        s = s[3..].trim_start_matches('_').to_string();
    }
    s = s.replace("::dyn", "::");
    s
}
