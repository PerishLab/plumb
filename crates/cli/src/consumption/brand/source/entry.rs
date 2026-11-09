use crate::shape::web::Syntax;
use tree_sitter::{Node, Parser};

const ENTRY: &str = r#"
import source from "virtual:perish/views";
import { Views } from "@perishlab/design";
import { mount } from "svelte";
const target = document.getElementById("root");
if (target !== null) {
    mount(Views, { target, props: { source } });
}
"#;

pub(super) fn verify(text: &str) -> Result<(), String> {
    if tokens(text)? != tokens(ENTRY)? {
        return Err(
            "Views entry is outside the qualified standard import/root/mount program".into(),
        );
    }
    Ok(())
}

fn tokens(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(text, None)
        .ok_or("Views entry was not parsed")?;
    if tree.root_node().has_error() {
        return Err("Views entry syntax is unqualified".into());
    }
    let mut found = Vec::new();
    Token(Syntax(text)).walk(tree.root_node(), &mut found)?;
    Ok(found)
}

struct Token<'a>(Syntax<'a>);

impl Token<'_> {
    fn walk(&self, node: Node<'_>, found: &mut Vec<(String, String)>) -> Result<(), String> {
        if node.kind() == "comment" {
            return Ok(());
        }
        if node.kind() == "string" {
            found.push((
                "string".into(),
                self.0
                    .string(node)
                    .ok_or("encoded Views entry string is unqualified")?,
            ));
            return Ok(());
        }
        if node.child_count() == 0 {
            found.push((node.kind().into(), self.0.text(node).into()));
            return Ok(());
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk(child, found)?;
        }
        Ok(())
    }
}
