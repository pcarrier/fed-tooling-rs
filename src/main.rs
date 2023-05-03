use std::{env, fs, rc::Rc, time::Instant};

use deno_core::{
    FastString::Static,
    JsRuntime,
    resolve_url,
    RuntimeOptions,
};
use deno_core::FastString::Owned;

include!(concat!(env!("OUT_DIR"), "/versions.rs"));

#[derive(serde::Serialize)]
struct Service {
    name: String,
    sdl: String,
    url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let services = env::args_os().skip(1).map(|a| Service {
        name: a.to_string_lossy().to_string(),
        sdl: fs::read_to_string(std::path::Path::new(&a)).unwrap().to_string(),
        url: None,
    }).collect::<Vec<_>>();
    let str = serde_json::to_string(&services)?;
    let invokation = String::into_boxed_str(["JSON.stringify(compose({services:", &str, "}))"].concat());

    for version in VERSIONS {
        let started = Instant::now();
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
            .execute_script("<main>", Owned(invokation.clone()))
            ?.open(runtime.v8_isolate())
            .to_rust_string_lossy(&mut runtime.handle_scope());
        println!("({:?}) {}: {}", started.elapsed(), version.name, res);
    }
    Ok(())
}

