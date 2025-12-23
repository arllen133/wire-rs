// wire-rs-cli/src/parser/mod.rs

pub mod blueprint;
pub mod import;
pub mod signature;

use self::blueprint::{Blueprint, BlueprintVisitor};
use self::import::ImportMapper;
use self::signature::{ProviderSignature, SignatureVisitor};
use crate::locator::FileLocator;
use std::path::Path;
use syn::visit::Visit;

pub struct Scanner {
    locator: FileLocator,
    blueprint: Blueprint,
    pub collected_providers: Vec<ProviderSignature>,
    // 映射表：类型路径 -> Provider 的完整签名
    provider_map: std::collections::HashMap<String, ProviderSignature>,
    // 防止递归死循环：正在处理的类型
    processing: std::collections::HashSet<String>,
}

impl Scanner {
    pub fn new(crate_root: std::path::PathBuf) -> Self {
        Self {
            locator: FileLocator::new(crate_root),
            blueprint: Blueprint::default(),
            collected_providers: Vec::new(),
            provider_map: std::collections::HashMap::new(),
            processing: std::collections::HashSet::new(),
        }
    }

    pub fn resolve_to_file(&self, path: &str) -> Option<std::path::PathBuf> {
        self.locator.resolve_to_file(path)
    }

    pub fn run(
        &mut self,
        entry_file: std::path::PathBuf,
        target_type: &str,
        _injector_fn: &str,
    ) -> Vec<ProviderSignature> {
        // 1. 加载蓝图，解析所有 Provider
        let entry_mapper = self.load_blueprint(&entry_file);

        // 2. Normalize target type using entry file's imports
        // e.g. "App" -> "crate::app::App"
        let normalized_target = {
            if target_type.contains("::") && target_type.starts_with("crate") {
                target_type.to_string()
            } else {
                let resolved = entry_mapper.resolve(target_type);
                if resolved.starts_with("self::") {
                    // Entry file is usually crate root or logic root.
                    // Assuming entry file path -> module path.
                    let logical_path = self
                        .locator
                        .file_to_logical(&entry_file)
                        .unwrap_or("crate".to_string());
                    resolved.replace("self", &logical_path)
                } else {
                    resolved
                }
            }
        };

        // 3. Resolve
        self.processing.clear();
        self.resolve_dependencies(&normalized_target);

        // 返回收集到的 providers
        self.collected_providers.clone()
    }

    /// 第一阶段：加载蓝图，预解析所有 Provider 的产出类型
    /// Returns the ImportMapper of the entry file for further use.
    pub fn load_blueprint(&mut self, entry_file: &Path) -> ImportMapper {
        let content = std::fs::read_to_string(entry_file).expect("Read entry failed");
        let ast = syn::parse_file(&content).expect("Parse entry failed");

        let mut visitor = BlueprintVisitor {
            blueprint: Blueprint::default(),
        };
        visitor.visit_file(&ast);

        // 获取初始 Mapper 解析入口文件的导入
        let mapper = ImportMapper::new(&ast);

        // 预爬取：确定每个 Provider 函数到底产出什么类型
        let raw_symbols = visitor.blueprint.providers.clone();

        for symbol in raw_symbols {
            let resolved = mapper.resolve(&symbol);
            self.resolve_and_cache_provider(entry_file, &resolved);
        }

        self.blueprint = visitor.blueprint;
        mapper
    }

    /// 辅助函数：找到 Provider 并记录它产出的类型
    fn resolve_and_cache_provider(&mut self, _current_file: &Path, symbol: &str) {
        // symbol e.g., "crate::db::provide_config"
        let logical_path = symbol.to_string(); // In our case, visited symbols are full paths

        if let Some(file_path) = self.locator.resolve_to_file(&logical_path) {
            let content = std::fs::read_to_string(&file_path).unwrap();
            let ast = syn::parse_file(&content).unwrap();
            let file_mapper = ImportMapper::new(&ast);

            // Determine module path from symbol
            // "crate::db::provide_config" -> "crate::db"
            // "crate::provide_foo" -> "crate"
            let module_path = if let Some(idx) = logical_path.rfind("::") {
                &logical_path[..idx]
            } else {
                "crate"
            };

            let target_fn_name = logical_path.split("::").last().unwrap().to_string();

            let mut sig_visitor = SignatureVisitor {
                target_symbol: target_fn_name.clone(),
                result: None,
            };
            sig_visitor.visit_file(&ast);

            if let Some(mut sig) = sig_visitor.result {
                sig.full_path = logical_path.clone();

                // Validate & Normalize Types
                // Helper to normalize a type string
                let normalize = |ty: &str| -> String {
                    if ty.contains("::") && ty.starts_with("crate") {
                        return ty.to_string();
                    }
                    let resolved = file_mapper.resolve(ty);
                    if resolved.starts_with("self::") {
                        resolved.replace("self", module_path)
                    } else {
                        resolved
                    }
                };

                sig.output_type = normalize(&sig.output_type);
                sig.stripped_inputs = sig.stripped_inputs.iter().map(|s| normalize(s)).collect();

                // Verify no collision or handle strict overwrite?
                // For now, overwrite is fine, but keys are now Full Paths!
                self.provider_map.insert(sig.output_type.clone(), sig);
            }
        }
    }

    /// 第二阶段：从 Injector 的目标类型开始递归构建图
    pub fn resolve_dependencies(&mut self, target_type: &str) {
        // target_type passed here must be Fully Qualified if we want to hit the map efficiently.
        // Or checking both?
        // Actually, the caller (Scanner::run) needs to normalize the initial target_type.

        let normalized_target = target_type; // Assumed normalized by caller

        if let Some(sig) = self.provider_map.get(normalized_target).cloned() {
            if self.collected_providers.iter().any(|p| p.name == sig.name) {
                return;
            }

            if self.processing.contains(normalized_target) {
                return;
            }

            self.processing.insert(normalized_target.to_string());

            // 先递归解决依赖
            for input_type in &sig.stripped_inputs {
                self.resolve_dependencies(input_type);
            }

            self.collected_providers.push(sig.clone());
            self.processing.remove(normalized_target);
        }
    }
}
