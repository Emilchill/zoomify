use std::{borrow::Cow, error::Error, path::Path};

use wasm_encoder::{ComponentNameSection, ComponentSection, CustomSection};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: cinnabar-zoomify-pack CORE.wasm COMPONENT.wasm".into());
    }

    let core = std::fs::read(&args[0])?;
    let mut component = wit_component::ComponentEncoder::default()
        .module(&core)?
        .validate(true)
        .encode()?;

    let mut name = ComponentNameSection::new();
    name.component("Zoomify");
    name.append_to_component(&mut component);
    append_section(
        &mut component,
        "description",
        b"Smooth, configurable zoom. Made by Emil Chill and Claude IA. Support me: https://ko-fi.com/emilchill",
    );
    append_section(&mut component, "version", env!("CARGO_PKG_VERSION").as_bytes());
    append_section(&mut component, "authors", b"Emil Chill and Claude IA");
    append_section(&mut component, "licenses", b"LGPL-3.0-or-later");
    append_section(
        &mut component,
        "cinnabar:extension/notices@1",
        include_bytes!("../../../../LICENSE"),
    );
    append_section(
        &mut component,
        "cinnabar:grants",
        include_bytes!("../grants.json"),
    );
    append_section(
        &mut component,
        "cinnabar:extension/icon@1",
        include_bytes!("../../../../assets/base-icon.png"),
    );

    // The menu, so the mod never sends its document at run time; it only patches values.
    let panel: serde_json::Value = serde_json::from_str(&cinnabar_mod_template::menu_panel())?;
    let mut surfaces = serde_json::Map::new();
    surfaces.insert(cinnabar_mod_template::MENU_ROUTE.to_owned(), panel["surface"].clone());
    let bundle = serde_json::json!({"panel": panel, "surfaces": surfaces});
    append_section(&mut component, mod_api::PANEL_BUNDLE_SECTION, &serde_json::to_vec(&bundle)?);

    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
        .validate_all(&component)?;
    for payload in wasmparser::Parser::new(0).parse_all(&component) {
        match payload? {
            wasmparser::Payload::ImportSection(imports) => {
                for import in imports.into_imports() {
                    if import?.module.starts_with("wasi") {
                        return Err("Cinnabar mods must not import WASI".into());
                    }
                }
            }
            wasmparser::Payload::ComponentImportSection(imports) => {
                for import in imports {
                    if import?.name.0.starts_with("wasi") {
                        return Err("Cinnabar mods must not import WASI".into());
                    }
                }
            }
            _ => {}
        }
    }

    let output = Path::new(&args[1]);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, &component)?;
    println!("Built Cinnabar component: {} bytes", component.len());
    Ok(())
}

fn append_section(component: &mut Vec<u8>, name: &str, data: &[u8]) {
    CustomSection {
        name: Cow::Borrowed(name),
        data: Cow::Borrowed(data),
    }
    .append_to_component(component);
}
