#[path = "src/matis/parsing/catalog.rs"]
mod catalog;

fn main() {
    println!("cargo:rerun-if-env-changed=TSLP_LANGUAGES");
    println!("cargo:rerun-if-env-changed=TSLP_LINK_MODE");
    println!("cargo:rerun-if-changed=src/matis/parsing/catalog.rs");
    let languages = std::env::var("TSLP_LANGUAGES").unwrap_or_default();
    let all = languages.trim().eq_ignore_ascii_case("all") || languages.trim() == "*";
    let configured: Vec<_> = languages.split(',').map(str::trim).collect();
    let missing: Vec<_> = catalog::LANGUAGES
        .iter()
        .copied()
        .filter(|language| *language != "jsx" && !all && !configured.contains(language))
        .collect();
    assert!(
        missing.is_empty() && std::env::var("TSLP_LINK_MODE").as_deref() == Ok("static"),
        "Matis requires all supported Tree-sitter grammars linked statically. \
         Build from the repository root so Cargo loads .cargo/config.toml, or set \
         TSLP_LINK_MODE=static and TSLP_LANGUAGES=python,javascript,typescript,tsx,java,kotlin,go,rust,elixir. \
         Missing grammars: {missing:?}"
    );
}
