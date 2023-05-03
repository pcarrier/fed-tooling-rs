use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{create_dir_all, write},
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::json;

struct Script<'a> {
    packages: &'a [(&'a str, &'a str)],
    index: &'a str,
}

struct Version<'a> {
    name: &'a str,
    compose: Script<'a>,
}

const FED1COMPOSE: &str = "
import {parse} from 'graphql/language/parser';
import {composeAndValidate} from '@apollo/federation';

interface Input {
    services: Array<{ name: string; sdl: string; url?: string }>,
}

interface Output {
    sdl?: string,
    errors: readonly Error[]
}

function rewriteNode(node) {
    let { kind, name, subgraph, loc } = node;
    loc = loc ? [loc.start, loc.end] : undefined;
    name = name ? name.value : undefined;
    return { kind, name, subgraph, loc };
}

function rewriteError(error) {
    const errors = error.errors ? error.errors.map(rewriteError) : undefined;
    const nodes = error.nodes ? error.nodes.map(rewriteNode) : undefined;
    return {...error, errors, nodes, source: undefined, stack: undefined};
}

globalThis.compose = function compose(
    input: Input
): Output {
    try {
        const definitions = input.services.map(({name, sdl, url}) => ({
            name,
            url,
            typeDefs: parse(sdl),
        }));

        const result = composeAndValidate(definitions);
        const errors = result.errors ? result.errors.map(rewriteError) : undefined;
        return {sdl: result.supergraphSdl, errors};
    } catch (e) {
        if (e instanceof Error) {
            return {errors: [{message: e.message}]};
        } else {
            return {errors: [{message: 'non-error thrown'}]};
        }
    }
}
";

const FED2COMPOSE: &str = "
import {parse} from 'graphql/language/parser';
import {composeServices} from '@apollo/composition';
import {CompositionHint} from '@apollo/composition/dist/hints';

interface Input {
    services: Array<{ name: string; sdl: string; url?: string }>,
}

interface Output {
    sdl?: string,
    hints?: readonly CompositionHint[]
    errors: readonly Error[]
}

function rewriteNode(node) {
    let { kind, name, subgraph, loc } = node;
    loc = loc ? [loc.start, loc.end] : undefined;
    name = name ? name.value : undefined;
    return { kind, name, subgraph, loc };
}

function rewriteError(error) {
    const errors = error.errors ? error.errors.map(rewriteError) : undefined;
    const nodes = error.nodes ? error.nodes.map(rewriteNode) : undefined;
    return {...error, errors, nodes, source: undefined, stack: undefined};
}

function rewriteHint(hint) {
    const nodes = hint.nodes ? hint.nodes.map(rewriteNode) : undefined;
    return {...hint, nodes};
}

globalThis.compose = function compose(
    input: Input
): Output {
    try {
        const definitions = input.services.map(({name, sdl, url}) => ({
            name,
            url,
            typeDefs: parse(sdl),
        }));

        const result = composeServices(definitions);
        const errors = result.errors ? result.errors.map(rewriteError) : undefined;
        const hints = result.hints ? result.hints.map(rewriteHint) : undefined;
        return {sdl: result.supergraphSdl, errors, hints};
    } catch (e) {
        if (e instanceof Error) {
            return {errors: [{message: e.message}]};
        } else {
            return {errors: [{message: 'non-error thrown'}]};
        }
    }
}
";

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
        bundle_script(&dir, "compose", &version.compose)?;
    }

    let out_dir = Path::new(&env::var_os("OUT_DIR").ok_or("OUT_DIR not set")?).to_owned();
    write(out_dir.join("versions.rs"), runtime_versions())?;

    Ok(())
}

fn runtime_versions() -> String {
    let list = VERSIONS.iter()
        .map(|t| format!(
            "Version {{ name: {:?}, compose: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/js/{}/compose/index.js\")) }},\n",
            t.name, t.name))
        .collect::<Vec<_>>().concat();
    [
        "struct Version<'a> {
            name: &'a str,
            compose: &'a str,
        }

        const VERSIONS: &[Version] = &[
    ",
        &list,
        "];"
    ].concat()
}

fn bundle_script(parent: &PathBuf, name: &str, script: &Script) -> Result<(), Box<dyn Error>> {
    let dir = parent.join(name);
    let src_dir = dir.join("src");
    create_dir_all(src_dir.clone())?;

    let dependencies = script.packages.into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<HashMap<String, String>>();

    write(dir.join("package.json"), json!({"dependencies": dependencies}).to_string())?;
    write(dir.join("tsconfig.json"), json!({"dependencies": dependencies}).to_string())?;

    let index = src_dir.join("index.ts");
    write(index, script.index)?;

    ensure(Command::new("npx")
        .arg("pnpm")
        .arg("install")
        .current_dir(dir.clone()))?;

    ensure(Command::new("npm")
        .arg("run")
        .arg("build")
        .arg("--")
        .arg(dir)
        .current_dir(parent))?;

    Ok(())
}
