use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use syn::{parse_macro_input, Ident, ItemFn, Path, ReturnType, LitStr, Token, parse::{Parse, ParseStream}};

mod graph;
mod models;

use graph::Graph;
use models::ProviderInfo;

// Simple struct to parse macro attributes like #[wire(wrappers = ["Arc", "Box"])]
struct WireAttr {
    wrappers: Vec<String>,
    file: String,
}

impl Parse for WireAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut wrappers = vec!["Arc".to_string(), "Box".to_string(), "Rc".to_string()];
        let mut file = "providers.json".to_string();
        
        if input.is_empty() {
            return Ok(WireAttr { wrappers, file });
        }

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident == "wrappers" {
                input.parse::<Token![=]>()?;
                let content;
                syn::bracketed!(content in input);
                let lit_strs: syn::punctuated::Punctuated<LitStr, Token![,]> = content.parse_terminated(|i| i.parse(), Token![,])?;
                wrappers = lit_strs.into_iter().map(|s| s.value()).collect();
            } else if ident == "file" {
                input.parse::<Token![=]>()?;
                let s: LitStr = input.parse()?;
                file = s.value();
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(WireAttr { wrappers, file })
    }
}

#[proc_macro_attribute]
pub fn provider(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    
    // Strip #[wire(...)] and #[bind(...)] attributes from parameters so they don't cause compile errors
    // since they are only for build-time scanning.
    for input in &mut func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            pat_type.attrs.retain(|attr| {
                !attr.path().is_ident("wire") && !attr.path().is_ident("inject")
            });
        }
    }
    
    quote! { #func }.into()
}

#[proc_macro_attribute]
pub fn wire(attr: TokenStream, item: TokenStream) -> TokenStream {
    let wire_attr = parse_macro_input!(attr as WireAttr);
    let wrappers = &wire_attr.wrappers;

    let input_fn = parse_macro_input!(item as ItemFn);
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;

    // 1. Read and parse provider data
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable not set");
    let providers_path = PathBuf::from(out_dir).join(&wire_attr.file);

    let providers_content = match fs::read_to_string(&providers_path) {
        Ok(content) => content,
        Err(e) => {
            let msg = format!(
                "Failed to read providers file at {:?}: {}",
                providers_path, e
            );
            return quote! { compile_error!(#msg); }.into();
        }
    };

    let all_providers: Vec<ProviderInfo> = match serde_json::from_str(&providers_content) {
        Ok(providers) => providers,
        Err(e) => {
            let msg = format!("Failed to deserialize providers file: {}", e);
            return quote! { compile_error!(#msg); }.into();
        }
    };

    // 2. Parse target type from function signature
    let (target_ty, is_target_result) = match &sig.output {
        ReturnType::Type(_, ty) => {
            let ty_str = ty.to_token_stream().to_string();
            let mut is_res = false;
            let mut inner_ty_str = ty_str.clone();

            if let syn::Type::Path(type_path) = &**ty {
                let last = type_path.path.segments.last().unwrap();
                if last.ident == "Result" {
                    is_res = true;
                    if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            inner_ty_str = inner.to_token_stream().to_string();
                        }
                    }
                }
            }
            (inner_ty_str, is_res)
        }
        ReturnType::Default => {
            return quote! { compile_error!("'#[wire]' function must have a return type."); }
                .into();
        }
    };

    let normalized_target = graph::normalize_type(&target_ty, wrappers);

    // 3. Build and resolve dependency graph
    let graph = match Graph::new(&all_providers, wrappers.clone()) {
        Ok(g) => g,
        Err(err_msg) => {
            return quote! { compile_error!(#err_msg); }.into();
        }
    };
    

    let target_key = if graph.nodes.contains_key(&normalized_target) {
        normalized_target.clone()
    } else {
        graph.nodes.keys()
            .find(|k| graph::is_match(&normalized_target, k) || graph::is_match(k, &normalized_target))
            .cloned()
            .unwrap_or(normalized_target.clone())
    };

    let sorted_providers = match graph.resolve(&target_key) {
        Ok(providers) => providers,
        Err(err_msg) => {
            return quote! { compile_error!(#err_msg); }.into();
        }
    };

    // 4. Generate the function body
    let mut var_map: HashMap<String, Ident> = HashMap::new();
    let mut actual_type_map: HashMap<String, String> = HashMap::new(); // Store original return type
    let mut generated_body = Vec::new();

    for provider in sorted_providers {
        let ret_ty_normalized = graph::normalize_type(&provider.ret, wrappers);
        
        let var_base = provider.ret.split('<').next().unwrap()
            .trim()
            .split("::").last().unwrap()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase();
        
        let var_name = format_ident!("{}_{}", var_base, var_map.len());
        var_map.insert(ret_ty_normalized.clone(), var_name.clone());
        actual_type_map.insert(ret_ty_normalized, provider.ret.clone());

        let provider_path: Path = syn::parse_str(&provider.path).unwrap();

        let mut arg_tokens = Vec::new();
        for arg in &provider.args {
            let lookup_ty = arg.from.as_ref().unwrap_or(&arg.ty);
            let arg_ty_normalized = graph::normalize_type(lookup_ty, wrappers);
            
            let arg_key = if var_map.contains_key(&arg_ty_normalized) {
                arg_ty_normalized
            } else {
                var_map.keys()
                    .find(|k| graph::is_match(&arg_ty_normalized, k) || graph::is_match(k, &arg_ty_normalized))
                    .cloned()
                    .unwrap_or(arg_ty_normalized)
            };

            let arg_var = var_map.get(&arg_key).expect(&format!(
                "BUG: Dependency '{}' not found in var_map",
                arg_key
            ));

            let provider_ret_ty = actual_type_map.get(&arg_key).unwrap().replace(" ", "");
            let arg_ty_clean = arg.ty.replace(" ", "");
            let is_arg_ref = arg_ty_clean.starts_with('&');
            
            // Check if we need to unwrap a smart pointer
            let mut needs_as_ref = false;
            for w in wrappers {
                if provider_ret_ty.contains(&format!("{}<", w)) && !arg_ty_clean.contains(&format!("{}<", w)) {
                    needs_as_ref = true;
                    break;
                }
            }

            // THE FIX: If types are different and we don't just need a simple as_ref (like Arc<T> -> &T),
            // OR if it's a Trait Object (contains 'dyn'), we need a local bridge to trigger Coersion.
            let mut final_arg_var = quote! { #arg_var };
            if !needs_as_ref && provider_ret_ty != arg_ty_clean && arg_ty_clean.contains("dyn") {
                let bridge_name = format_ident!("{}_bridge_{}", arg_var, arg_tokens.len());
                let expected_ty_base = if is_arg_ref {
                    // Strip one '&'
                    &arg.ty[1..]
                } else {
                    &arg.ty
                };
                let expected_ty: syn::Type = syn::parse_str(expected_ty_base).unwrap();
                
                generated_body.push(quote! {
                    let #bridge_name: #expected_ty = #arg_var.clone();
                });
                final_arg_var = quote! { #bridge_name };
            }

            if is_arg_ref {
                if needs_as_ref {
                    arg_tokens.push(quote! { #final_arg_var.as_ref() });
                } else {
                    arg_tokens.push(quote! { &#final_arg_var });
                }
            } else {
                if needs_as_ref {
                    arg_tokens.push(quote! { #final_arg_var.as_ref().clone() });
                } else {
                    arg_tokens.push(quote! { #final_arg_var.clone() });
                }
            }
        }

        let try_op = if provider.is_result {
            if !is_target_result {
                return quote! { compile_error!("Target function must return Result because some providers return Result."); }.into();
            }
            quote! { ? }
        } else {
            quote! { }
        };

        generated_body.push(quote! {
            let #var_name = #provider_path(#(#arg_tokens),*) #try_op;
        });

        for b in &provider.bindings {
            let ty_b_normalized = graph::normalize_type(b, wrappers);
            let b_type: syn::Type = syn::parse_str(b).unwrap();
            let var_name_binding = format_ident!("{}_as_{}", var_base, ty_b_normalized);
            
            // Generate a bridging variable to trigger coercion
            generated_body.push(quote! {
                let #var_name_binding: #b_type = #var_name.clone();
            });

            var_map.insert(ty_b_normalized.clone(), var_name_binding);
            actual_type_map.insert(ty_b_normalized, b.to_string());
        }
    }

    let final_var = var_map
        .get(&target_key)
        .expect("BUG: Final target not in var_map");

    let final_return = if is_target_result {
        quote! { Ok(#final_var) }
    } else {
        quote! { #final_var }
    };

    let expanded = quote! {
        #vis #sig {
            #(#generated_body)*
            #final_return
        }
    };

    // Uncomment for debugging
    // println!("--- WIRE MACRO EXPANSION for {} ---", sig.ident);
    // println!("{}", expanded.to_string());
    // println!("-----------------------------------");

    TokenStream::from(expanded)
}
