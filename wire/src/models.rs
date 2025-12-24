use serde::{Deserialize, Serialize};

// These structs mirror the ones in `wire-scanner`.
// They are used to deserialize the `providers.json` file.

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
