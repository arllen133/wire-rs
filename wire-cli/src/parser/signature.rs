use quote::ToTokens;
use syn::{ItemFn, ReturnType, visit::Visit};

#[derive(Debug, Clone)]
pub struct ProviderSignature {
    pub full_path: String,            // 完整的调用路径，如 crate::db::provide_pool
    pub name: String,                 // 函数名，如 provide_pool
    pub inputs: Vec<String>,          // 原始参数，如 "cfg: &Config"
    pub stripped_inputs: Vec<String>, // 纯类型参数，如 "Config"
    pub output_type: String,          // 产出类型，如 "Pool"
    pub is_result: bool,
}

pub struct SignatureVisitor {
    pub target_symbol: String,
    pub result: Option<ProviderSignature>,
}

impl<'ast> Visit<'ast> for SignatureVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if i.sig.ident == self.target_symbol {
            self.result = Some(self.parse_sig(&i.sig));
        }
    }
}

impl SignatureVisitor {
    fn parse_sig(&self, sig: &syn::Signature) -> ProviderSignature {
        let (output_type, is_result) = match &sig.output {
            ReturnType::Default => ("()".to_string(), false),
            ReturnType::Type(_, ty) => {
                let s = ty.to_token_stream().to_string().replace(" ", "");
                if s.starts_with("Result<") {
                    let inner = s
                        .trim_start_matches("Result<")
                        .split(',')
                        .next()
                        .unwrap()
                        .to_string();
                    (inner, true)
                } else {
                    (s, false)
                }
            }
        };

        let stripped_inputs = sig
            .inputs
            .iter()
            .map(|arg| {
                let s = arg.to_token_stream().to_string();
                s.split(':')
                    .last()
                    .unwrap()
                    .trim()
                    .replace("&", "")
                    .replace(" ", "")
            })
            .collect();

        ProviderSignature {
            full_path: String::new(), // 由外部 Scanner 填充
            name: sig.ident.to_string(),
            inputs: sig
                .inputs
                .iter()
                .map(|a| a.to_token_stream().to_string())
                .collect(),
            stripped_inputs,
            output_type,
            is_result,
        }
    }
}
