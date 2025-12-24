use anyhow::Result;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use syn::{File, FnArg, Item, Pat};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProviderArgument {
    pub name: String,
    pub ty: String,
    pub from: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProviderInfo {
    pub path: String,
    pub args: Vec<ProviderArgument>,
    pub ret: String,
    pub is_result: bool,
    pub bindings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileScanResult {
    pub mtime: SystemTime,
    pub providers: Vec<ProviderInfo>,
}

pub type ScanCache = HashMap<PathBuf, FileScanResult>;

/// Scans the source directory and generates a JSON file containing all found providers.
/// This is intended to be called from a build script.
pub fn generate(src_dir: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Result<()> {
    let src_dir = src_dir.as_ref();
    let output_path = output_path.as_ref();
    let out_dir = output_path.parent().expect("output_path must have a parent");
    let file_stem = output_path.file_stem().and_then(|s| s.to_str()).unwrap_or("providers");
    let cache_path = out_dir.join(format!("{}_cache.json", file_stem));

    eprintln!("wire-build: generating providers from {:?} to {:?}", src_dir, output_path);
    let cache = scan(src_dir, &cache_path)?;
    let all_providers: Vec<ProviderInfo> = cache
        .values()
        .flat_map(|result| result.providers.clone())
        .collect();

    fs::write(output_path, serde_json::to_string_pretty(&all_providers)?)?;

    Ok(())
}

/// Scans a source directory for provider functions, using a cache for incremental processing.
pub fn scan(src_dir: &Path, cache_path: &Path) -> Result<ScanCache> {
    let mut cache: ScanCache = if cache_path.exists() {
        let cache_content = fs::read(cache_path)?;
        serde_json::from_slice(&cache_content).unwrap_or_default()
    } else {
        HashMap::new()
    };

    let pattern = format!("{}/**/*.rs", src_dir.to_str().unwrap());
    let mut seen_files = HashSet::new();

    for entry in glob::glob(&pattern)? {
        let path = entry?;
        seen_files.insert(path.clone());
        let mtime = fs::metadata(&path)?.modified()?;

        if let Some(cached_result) = cache.get(&path) {
            if cached_result.mtime == mtime {
                continue;
            }
        }

        eprintln!("wire-build: Scanning file: {:?}", &path);
        let content = fs::read_to_string(&path)?;
        let ast = match syn::parse_file(&content) {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("wire-build: Warning: Skipping file {:?} due to syntax error: {}", path, e);
                cache.remove(&path); // Ensure stale data is removed from cache
                continue;
            }
        };

        // Construct module path from file path
        let mod_path = path_to_module_path(&path, src_dir);

        let providers = parse_providers_from_ast(&ast, &mod_path)?;

        let result = FileScanResult { mtime, providers };
        cache.insert(path, result);
    }

    // Remove files that no longer exist from cache
    cache.retain(|path, _| seen_files.contains(path));

    fs::write(cache_path, serde_json::to_string_pretty(&cache)?)?;

    Ok(cache)
}

fn path_to_module_path(path: &Path, src_dir: &Path) -> String {
    path.strip_prefix(src_dir)
        .unwrap()
        .with_extension("")
        .to_str()
        .unwrap()
        .replace("/", "::")
        .replace("\\", "::")
}

/// Parses a syn::File AST to find functions with the `#[provider]` attribute.
fn parse_providers_from_ast(ast: &File, mod_path: &str) -> Result<Vec<ProviderInfo>> {
    let mut providers = Vec::new();

    for item in &ast.items {
        if let Item::Fn(func) = item {
            let is_provider = func.attrs.iter().any(|attr| {
                attr.path()
                    .segments
                    .last()
                    .map_or(false, |segment| segment.ident == "provider")
            });

            if is_provider {
                let fn_name = func.sig.ident.to_string();
                let path = if mod_path == "main" || mod_path == "lib" {
                    format!("crate::{}", fn_name)
                } else if mod_path.ends_with("::mod") {
                    let base_mod = mod_path.strip_suffix("::mod").unwrap();
                    format!("crate::{}::{}", base_mod, fn_name)
                } else {
                    format!("crate::{}::{}", mod_path, fn_name)
                };

                let args = func
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|arg| {
                        if let FnArg::Typed(pat_type) = arg {
                            let name = if let Pat::Ident(pat_ident) = &*pat_type.pat {
                                pat_ident.ident.to_string()
                            } else {
                                "_".to_string()
                            };
                            let ty = pat_type.ty.to_token_stream().to_string();
                            let from = pat_type.attrs.iter().find_map(|attr| {
                                if attr.path().is_ident("inject") {
                                    if let Ok(ty) = attr.parse_args::<syn::Type>() {
                                        return Some(ty.to_token_stream().to_string());
                                    }
                                }
                                if attr.path().is_ident("wire") {
                                    if let Ok(list) = attr.meta.require_list() {
                                        if let Ok(nested) = list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated) {
                                            for meta in nested {
                                                if let syn::Meta::NameValue(nv) = meta {
                                                    if nv.path.is_ident("from") {
                                                        if let syn::Expr::Lit(expr_lit) = &nv.value {
                                                            if let syn::Lit::Str(lit) = &expr_lit.lit {
                                                                return Some(lit.value());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                None
                            });
                            Some(ProviderArgument { name, ty, from })
                        } else {
                            None
                        }
                    })
                    .collect();

                let (ret, is_result) = if let syn::ReturnType::Type(_, ty) = &func.sig.output {
                    let ty_str = ty.to_token_stream().to_string();
                    
                    // Simple check for Result patterns
                    if let syn::Type::Path(type_path) = &**ty {
                        let last = type_path.path.segments.last().unwrap();
                        if last.ident == "Result" {
                            // Extract T from Result<T, E> or anyhow::Result<T>
                            if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                    (inner_ty.to_token_stream().to_string(), true)
                                } else {
                                    (ty_str, true) // Assume Result but couldn't unwrap
                                }
                            } else {
                                (ty_str, true)
                            }
                        } else {
                            (ty_str, false)
                        }
                    } else {
                        (ty_str, false)
                    }
                } else {
                    ("()".to_string(), false)
                };

                let bindings = func.attrs.iter().filter_map(|attr| {
                    if attr.path().segments.last().map_or(false, |s| s.ident == "bind") {
                        if let Ok(nested) = attr.parse_args::<syn::Type>() {
                             return Some(nested.to_token_stream().to_string());
                        }
                    }
                    None
                }).collect();

                providers.push(ProviderInfo { path, args, ret, is_result, bindings });
            }
        }
    }

    Ok(providers)
}
