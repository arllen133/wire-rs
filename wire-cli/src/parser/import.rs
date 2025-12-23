use std::collections::HashMap;
use syn::{ItemUse, UseTree, visit::Visit};

pub struct ImportMapper {
    // "Pool" -> "crate::db::Pool"
    pub symbol_map: HashMap<String, String>,
}

impl ImportMapper {
    pub fn new(file: &syn::File) -> Self {
        let mut mapper = Self {
            symbol_map: HashMap::new(),
        };
        mapper.visit_file(file);
        mapper
    }

    pub fn resolve(&self, symbol: &str) -> String {
        self.symbol_map
            .get(symbol)
            .cloned()
            .unwrap_or_else(|| format!("self::{}", symbol))
    }
}

impl<'ast> Visit<'ast> for ImportMapper {
    fn visit_item_use(&mut self, i: &'ast ItemUse) {
        self.extract_tree(&i.tree, String::new());
    }
}

impl ImportMapper {
    fn extract_tree(&mut self, tree: &UseTree, prefix: String) {
        match tree {
            UseTree::Path(p) => {
                let new_prefix = if prefix.is_empty() {
                    p.ident.to_string()
                } else {
                    format!("{}::{}", prefix, p.ident)
                };
                self.extract_tree(&*p.tree, new_prefix);
            }
            UseTree::Group(g) => {
                for item in &g.items {
                    self.extract_tree(item, prefix.clone());
                }
            }
            UseTree::Name(n) => {
                let name = n.ident.to_string();
                self.symbol_map
                    .insert(name.clone(), format!("{}::{}", prefix, name));
            }
            UseTree::Rename(r) => {
                self.symbol_map
                    .insert(r.rename.to_string(), format!("{}::{}", prefix, r.ident));
            }
            UseTree::Glob(_) => {}
        }
    }
}
