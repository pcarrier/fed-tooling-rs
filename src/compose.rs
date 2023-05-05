use std::env;
use std::fs::read_to_string;
use std::path::Path;
use deno_core::futures::executor::block_on;
use fedtooling::{compose, find_version, Service};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let version_str = env::args().nth(1).expect("no version provided");
    let version = find_version(version_str.as_str()).expect("version not found");

    let mut services = vec![];
    for arg in env::args().skip(2) {
        let mut parts = arg.split(':');
        let name = parts.next().expect("no name provided").to_string();
        let sdl = read_to_string(Path::new(parts.next().expect("no file provided")))?;
        let url_offset = name.len() + 1 + sdl.len() + 1;
        let url = if arg.len() >= url_offset { Some(arg[url_offset..].to_string()) } else { None };
        services.push(Service { name, sdl, url });
    }

    let (result, str) = block_on(compose(version, &services))?;
    println!("Raw response: {}", str);
    if let Some(errs) = result.errors {
        for err in errs {
            println!("{:?}", err);
        }
        return Err("errors".into());
    }

    if let Some(hints) = result.hints {
        for hint in hints {
            println!("{:?}", hint);
        }
    }

    if let Some(sdl) = result.sdl {
        println!("SDL:\n{}", sdl);
    }

    Ok(())
}
