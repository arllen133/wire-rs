use std::collections::HashMap;
use syn::{
    Expr, ItemFn,
    visit::{self, Visit},
};

#[derive(Default)]
pub struct Blueprint {
    pub providers: Vec<String>,             // 收集到的所有 Provider 路径
    pub injectors: HashMap<String, String>, // Injector名 -> 关联配置函数名
}

pub struct BlueprintVisitor {
    pub blueprint: Blueprint,
}

impl<'ast> Visit<'ast> for BlueprintVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        // 1. 识别 Injector
        let injector_attr = i.attrs.iter().find(|a| a.path().is_ident("injector"));
        if let Some(attr) = injector_attr {
            let fn_name = i.sig.ident.to_string();
            // 提取 #[injector(config_fn)] 里的参数
            let config_fn = attr
                .parse_args::<syn::Ident>()
                .map(|id| id.to_string())
                .map(|id| id.to_string())
                .unwrap_or_default();
            self.blueprint.injectors.insert(fn_name, config_fn);
            // Don't return, we need to visit the body to find the tuple!
        }

        // 2. 识别配置函数（假设我们分析所有函数，寻找元组形式的引用）
        // 实际逻辑中，我们可以只分析被 injector 关联的那个函数
        visit::visit_item_fn(self, i);
    }

    // 在函数体内部寻找表达式
    fn visit_expr_tuple(&mut self, i: &'ast syn::ExprTuple) {
        for elem in &i.elems {
            if let Expr::Path(p) = elem {
                // 将路径转为字符串，如 "UserService::new" 或 "provide_pool"
                let path_str = quote::quote!(#p).to_string().replace(" ", "");
                self.blueprint.providers.push(path_str);
            }
        }
    }
}
