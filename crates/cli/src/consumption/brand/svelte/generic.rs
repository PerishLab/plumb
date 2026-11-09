use crate::shape::web::Syntax;
use tree_sitter::Parser;

pub(super) fn read(value: &str) -> Result<(), String> {
    if !value.starts_with(['\'', '"']) || value.as_bytes().last() != value.as_bytes().first() {
        return Err("dynamic Svelte generic parameters are unqualified".into());
    }
    let content = value
        .get(1..value.len().saturating_sub(1))
        .ok_or("missing generic parameters")?;
    let source = format!("type Scope<{content}> = unknown;");
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(&source, None)
        .ok_or("TypeScript generic parameters were not parsed")?;
    if tree.root_node().has_error() {
        return Err("unsupported Svelte generic parameter syntax".into());
    }
    let syntax = Syntax(&source);
    let children = syntax.children(tree.root_node());
    let [alias] = children.as_slice() else {
        return Err("Svelte generics must contain only type parameters".into());
    };
    let name = alias
        .child_by_field_name("name")
        .map(|node| syntax.text(node));
    let body = alias
        .child_by_field_name("value")
        .map(|node| syntax.text(node));
    let parameters = alias
        .child_by_field_name("type_parameters")
        .map(|node| syntax.text(node));
    if parameters != Some(format!("<{content}>").as_str()) {
        return Err("Svelte generics must occupy the exact type parameter span".into());
    }
    if alias.kind() != "type_alias_declaration" || name != Some("Scope") || body != Some("unknown")
    {
        return Err("Svelte generics changed the enclosing type declaration".into());
    }
    Ok(())
}
