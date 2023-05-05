use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{create_dir_all, write},
    path::{Path, PathBuf},
    process::Command,
};
use std::fs::read_to_string;
use std::rc::Rc;

use deno_core::{JsRuntime, resolve_url, RuntimeOptions};
use deno_core::FastString::Owned;
use deno_core::futures::executor::block_on;
use deno_core::v8::StartupData;
use serde_json::json;

struct Script<'a> {
    packages: &'a [(&'a str, &'a str)],
    index: &'a str,
}

struct Version<'a> {
    name: &'a str,
    compose: Script<'a>,
}

const FED1COMPOSE: &str = include_str!("js/fed1compose.ts");
const FED2COMPOSE: &str = include_str!("js/fed2compose.ts");

const VERSIONS: &[Version] = &[
    Version {
        name: "1.0",
        compose: Script {
            packages: &[
                ("@apollo/federation", "0.23.2"),
                ("@apollo/query-planner", "0.1.2"),
                ("apollo-graphql", "0.9.5"),
                ("graphql", "15.7.2"),
            ],
            index: FED1COMPOSE,
        },
    },
    Version {
        name: "1.1",
        compose: Script {
            packages: &[
                ("@apollo/federation", "0.29.1"),
                ("@apollo/query-planner", "0.3.2"),
                ("apollo-graphql", "0.9.5"),
                ("graphql", "15.7.2"),
            ],
            index: FED1COMPOSE,
        },
    },
    Version {
        name: "2.0",
        compose: Script {
            packages: &[
                ("@apollo/composition", "2.0.5"),
                ("@apollo/core-schema", "0.3.0"),
                ("@apollo/federation-internals", "2.0.5"),
                ("@apollo/query-graphs", "2.0.5"),
                ("@apollo/query-planner", "2.0.5"),
                ("graphql", "16.3.0"),
            ],
            index: FED2COMPOSE,
        },
    },
    Version {
        name: "2.1",
        compose: Script {
            packages: &[
                ("@apollo/composition", "2.1.4"),
                ("@apollo/federation-internals", "2.1.4"),
                ("@apollo/query-graphs", "2.1.4"),
                ("@apollo/query-planner", "2.1.4"),
                ("graphql", "16.5.0"),
            ],
            index: FED2COMPOSE,
        },
    },
    Version {
        name: "2.2",
        compose: Script {
            packages: &[
                ("@apollo/composition", "2.2.1"),
                ("@apollo/federation-internals", "2.2.1"),
                ("@apollo/query-graphs", "2.2.1"),
                ("@apollo/query-planner", "2.2.1"),
                ("graphql", "16.5.0"),
            ],
            index: FED2COMPOSE,
        },
    },
    Version {
        name: "2.3",
        compose: Script {
            packages: &[
                ("@apollo/composition", "2.3.0"),
                ("@apollo/federation-internals", "2.3.0"),
                ("@apollo/query-graphs", "2.3.0"),
                ("@apollo/query-planner", "2.3.0"),
                ("graphql", "16.5.0"),
            ],
            index: FED2COMPOSE,
        },
    },
    Version {
        name: "2.4",
        compose: Script {
            packages: &[
                ("@apollo/composition", "2.4.3"),
                ("@apollo/federation-internals", "2.4.3"),
                ("@apollo/query-graphs", "2.4.3"),
                ("@apollo/query-planner", "2.4.3"),
                ("graphql", "16.5.0"),
            ],
            index: FED2COMPOSE,
        },
    },
];

fn ensure(es: &mut Command) -> Result<(), Box<dyn Error>> {
    let status = es.status()?;
    if !status.success() {
        return Err(format!("command {:?} failed with status {}", es.get_args(), status).into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let js_dir = env::current_dir()?.join("js");

    ensure(Command::new("npm")
        .arg("ci")
        .current_dir(js_dir.clone()))?;
    // pnpm is thereby accessible

    for version in VERSIONS {
        let dir = js_dir.join(version.name);
        bundle_script(&dir, "compose", &version, &version.compose)?;
    }

    let out_dir = Path::new(&env::var_os("OUT_DIR").ok_or("OUT_DIR not set")?).to_owned();
    write(out_dir.join("versions.rs"), runtime_versions())?;

    Ok(())
}

fn runtime_versions() -> String {
    let list = VERSIONS.iter()
        .map(|t| format!(
            "Version {{ name: {:?}, compose: include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/js/{}/compose/snap.bin\")) }},\n",
            t.name, t.name))
        .collect::<Vec<_>>().concat();
    [
        "pub struct Version<'a> {
            name: &'a str,
            compose: &'a [u8],
        }

        const VERSIONS: &[Version] = &[
    ",
        &list,
        "];"
    ].concat()
}

fn bundle_script(parent: &PathBuf, name: &str, version: &Version, script: &Script) -> Result<(), Box<dyn Error>> {
    let dir = parent.join(name);
    let src_dir = dir.join("src");
    create_dir_all(src_dir.clone())?;

    let dependencies = script.packages.into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<HashMap<String, String>>();

    write(dir.join("package.json"), json!({"dependencies": dependencies}).to_string())?;
    write(dir.join("tsconfig.json"), json!({"dependencies": dependencies}).to_string())?;

    let index_ts = src_dir.join("index.ts");
    write(index_ts, script.index)?;

    ensure(Command::new("npx")
        .arg("pnpm")
        .arg("install")
        .current_dir(dir.clone()))?;

    ensure(Command::new("npm")
        .arg("run")
        .arg("build")
        .arg("--")
        .arg(dir.clone())
        .current_dir(parent))?;

    let index_js = dir.join("index.js");
    write(dir.join("snap.bin"), snapshot(index_js, version)?)?;

    Ok(())
}

fn snapshot(path: PathBuf, version: &Version) -> Result<StartupData, Box<dyn Error>> {
    let str = read_to_string(path)?;

    let mut runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![
            deno_webidl::deno_webidl::init_ops(),
            deno_url::deno_url::init_ops(),
        ],
        will_snapshot: true,
        ..Default::default()
    });

    let specifier = resolve_url(&format!("file:///{}.js", version.name))?;
    let module = block_on(runtime
        .load_main_module(&specifier, Some(Owned(str.into_boxed_str()))))?;
    let mod_load = runtime.mod_evaluate(module);
    block_on(runtime.run_event_loop(false))?;
    block_on(mod_load)??;

    let snap = runtime.snapshot();

    Ok(snap)
}
