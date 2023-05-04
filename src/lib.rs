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

#[derive(serde::Deserialize, PartialEq, Eq, Debug)]
pub struct Result {
    sdl: Option<String>,
    hints: Option<Vec<Hint>>,
    errors: Option<Vec<Error>>,
}

#[derive(serde::Deserialize, PartialEq, Eq, Debug)]
pub struct Node {
    kind: String,
    name: String,
    subgraph: Option<String>,
    loc: Option<(u32, u32)>,
}

#[derive(serde::Deserialize, PartialEq, Eq, Debug)]
pub struct Hint {
    // FIXME: there's more to a hint.
    nodes: Option<Vec<Node>>
}

#[derive(serde::Deserialize, PartialEq, Eq, Debug)]
pub struct Error {
    message: String,
    errors: Option<Vec<Error>>,
    nodes: Option<Vec<Node>>
}

pub async fn compose(version: &str, services: &[Service]) -> std::result::Result<(Result, String), Box<dyn std::error::Error>> {
    let str = serde_json::to_string(&services)?;
    let invokation = String::into_boxed_str(["JSON.stringify(compose({services:", &str, "}))"].concat());
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

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compose() {
        let services = vec![
            Service {
                name: "foo".to_string(),
                sdl: "type Query { foo: Foo } type Foo @key(fields:\"id\") { id: ID! }".to_string(),
                url: Some("http://foo".to_string()),
            },
            Service {
                name: "bar".to_string(),
                sdl: "type Bar @key(fields:\"id\") { id: ID! }".to_string(),
                url: Some("http://bar".to_string()),
            }
        ];
        let out = compose("1.0", &services).await;
        let (res, str) = out.expect("compose failed");
        assert_eq!(str, "{\"sdl\":\"schema\\n  @core(feature: \\\"https://specs.apollo.dev/core/v0.1\\\"),\\n  @core(feature: \\\"https://specs.apollo.dev/join/v0.1\\\")\\n{\\n  query: Query\\n}\\n\\ndirective @core(feature: String!) repeatable on SCHEMA\\n\\ndirective @join__field(graph: join__Graph, requires: join__FieldSet, provides: join__FieldSet) on FIELD_DEFINITION\\n\\ndirective @join__type(graph: join__Graph!, key: join__FieldSet) repeatable on OBJECT | INTERFACE\\n\\ndirective @join__owner(graph: join__Graph!) on OBJECT | INTERFACE\\n\\ndirective @join__graph(name: String!, url: String!) on ENUM_VALUE\\n\\ntype Bar\\n  @join__owner(graph: BAR)\\n  @join__type(graph: BAR, key: \\\"id\\\")\\n{\\n  id: ID! @join__field(graph: BAR)\\n}\\n\\ntype Foo\\n  @join__owner(graph: FOO)\\n  @join__type(graph: FOO, key: \\\"id\\\")\\n{\\n  id: ID! @join__field(graph: FOO)\\n}\\n\\nscalar join__FieldSet\\n\\nenum join__Graph {\\n  BAR @join__graph(name: \\\"bar\\\" url: \\\"http://bar\\\")\\n  FOO @join__graph(name: \\\"foo\\\" url: \\\"http://foo\\\")\\n}\\n\\ntype Query {\\n  foo: Foo @join__field(graph: FOO)\\n}\\n\"}");
        assert_eq!(res.hints, None);
        assert_eq!(res.errors, None);
        assert_eq!(res.sdl, Some("schema\n  @core(feature: \"https://specs.apollo.dev/core/v0.1\"),\n  @core(feature: \"https://specs.apollo.dev/join/v0.1\")\n{\n  query: Query\n}\n\ndirective @core(feature: String!) repeatable on SCHEMA\n\ndirective @join__field(graph: join__Graph, requires: join__FieldSet, provides: join__FieldSet) on FIELD_DEFINITION\n\ndirective @join__type(graph: join__Graph!, key: join__FieldSet) repeatable on OBJECT | INTERFACE\n\ndirective @join__owner(graph: join__Graph!) on OBJECT | INTERFACE\n\ndirective @join__graph(name: String!, url: String!) on ENUM_VALUE\n\ntype Bar\n  @join__owner(graph: BAR)\n  @join__type(graph: BAR, key: \"id\")\n{\n  id: ID! @join__field(graph: BAR)\n}\n\ntype Foo\n  @join__owner(graph: FOO)\n  @join__type(graph: FOO, key: \"id\")\n{\n  id: ID! @join__field(graph: FOO)\n}\n\nscalar join__FieldSet\n\nenum join__Graph {\n  BAR @join__graph(name: \"bar\" url: \"http://bar\")\n  FOO @join__graph(name: \"foo\" url: \"http://foo\")\n}\n\ntype Query {\n  foo: Foo @join__field(graph: FOO)\n}\n".to_string()));
    }
}
