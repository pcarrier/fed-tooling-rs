use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::{create_dir_all, write};
use std::process::Command;

use serde_json::json;

struct Version<'a> {
    name: &'a str,
    packages: &'a [(&'a str, &'a str)],
    compose: &'a str,
}

const VERSIONS: &[Version] = &[
    Version {
        name: "1.0",
        packages: &[
            ("@apollo/federation", "0.23.2"),
            ("@apollo/query-planner", "0.1.2"),
            ("apollo-graphql", "0.9.5"),
            ("graphql", "15.7.2"),
        ],
        compose: "export default function() { }",
    },
    Version {
        name: "1.1",
        packages: &[
            ("@apollo/federation", "0.29.1"),
            ("@apollo/query-planner", "0.3.2"),
            ("apollo-graphql", "0.9.5"),
            ("graphql", "15.7.2"),
        ],
        compose: "export default function() { }",
    },
    Version {
        name: "2.0",
        packages: &[
            ("@apollo/composition", "2.0.5"),
            ("@apollo/core-schema", "0.3.0"),
            ("@apollo/federation-internals", "2.0.5"),
            ("@apollo/query-graphs", "2.0.5"),
            ("@apollo/query-planner", "2.0.5"),
            ("graphql", "16.3.0"),
        ],
        compose: "export default function() { }",
    },
    Version {
        name: "2.1",
        packages: &[
            ("@apollo/composition", "2.1.4"),
            ("@apollo/federation-internals", "2.1.4"),
            ("@apollo/query-graphs", "2.1.4"),
            ("@apollo/query-planner", "2.1.4"),
            ("graphql", "16.5.0"),
        ],
        compose: "export default function() { }",
    },
    Version {
        name: "2.2",
        packages: &[
            ("@apollo/composition", "2.2.1"),
            ("@apollo/federation-internals", "2.2.1"),
            ("@apollo/query-graphs", "2.2.1"),
            ("@apollo/query-planner", "2.2.1"),
            ("graphql", "16.5.0"),
        ],
        compose: "export default function() { }",
    },
    Version {
        name: "2.3",
        packages: &[
            ("@apollo/composition", "2.3.0"),
            ("@apollo/federation-internals", "2.3.0"),
            ("@apollo/query-graphs", "2.3.0"),
            ("@apollo/query-planner", "2.3.0"),
            ("graphql", "16.5.0"),
        ],
        compose: "export default function() { }",
    },
];

fn main() -> Result<(), Box<dyn Error>> {
    let js_dir = env::current_dir()?.join("js");
    Command::new("npm")
        .arg("install")
        .current_dir(js_dir.clone())
        .spawn()?
        .wait()?;

    for version in VERSIONS {
        let dir = js_dir.join(version.name);
        create_dir_all(dir.clone())?;
        let dependencies = version.packages.into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>();
        let package_json = json!({
            "name": "federation",
            "dependencies": dependencies,
        });
        write(dir.join("package.json"), package_json.to_string())?;
        let src_dir = dir.join("src");
        create_dir_all(src_dir.clone())?;
        write(src_dir.join("compose.js"), version.compose)?;
        Command::new("npm")
            .arg("install")
            .current_dir(dir.clone())
            .spawn()?
            .wait()?;
    }

    Ok(())
}
