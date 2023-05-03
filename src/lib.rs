use std::rc::Rc;

use deno_core::{
    FastString::Static,
    JsRuntime,
    resolve_url,
    RuntimeOptions,
};
use deno_core::FastString::Owned;

include!(concat!(env!("OUT_DIR"), "/versions.rs"));

#[derive(serde::Serialize)]
pub struct Service {
    name: String,
    sdl: String,
    url: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct Result {
    sdl: Option<String>,
    hints: Option<Vec<Hint>>,
    errors: Option<Vec<Error>>,
}

#[derive(serde::Deserialize)]
pub struct Node {
    kind: String,
    name: String,
    subgraph: String,
    loc: Option<(u32, u32)>,
}

#[derive(serde::Deserialize)]
pub struct Hint {
    nodes: Option<Vec<Node>>
}

#[derive(serde::Deserialize)]
pub struct Error {
    message: String,
    errors: Option<Vec<Error>>,
    nodes: Option<Vec<Node>>
}

pub async fn compose(version: &str, services: &[Service]) -> std::result::Result<(Result, String), Box<dyn std::error::Error>> {
    let str = serde_json::to_string(&services)?;
    let invokation = String::into_boxed_str(["JSON.stringify(compose({services:", &str, "))"].concat());
    let version = VERSIONS.iter().find(|v| v.name == version).ok_or(format!("version {} not found", version))?;

    let mut runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![
            deno_webidl::deno_webidl::init_ops_and_esm(),
            deno_url::deno_url::init_ops_and_esm(),
        ],
        ..Default::default()
    });
    let specifier = resolve_url(&format!("file:///{}.js", version.name))?;
    let module = runtime.load_main_module(&specifier, Some(Static(version.compose))).await?;
    let mod_load = runtime.mod_evaluate(module);
    runtime.run_event_loop(false).await?;
    mod_load.await??;
    let res = runtime
        .execute_script("<main>", Owned(invokation.clone()))?;
    let str = res.open(runtime.v8_isolate())
        .to_rust_string_lossy(&mut runtime.handle_scope());
    Ok((serde_json::from_str(str.as_str())?, str))
}
