use super::{Activation, Setup};
fn active(options: serde_json::Value) -> Activation {
    Activation::Active(Setup {
        options,
        base: "/".into(),
        output: "dist".into(),
        problem: None,
    })
}

fn config(plugin: &str) -> String {
    format!(
        r#"import {{ design as crest }} from "@perishlab/design/vite";
import {{ defineConfig as configure }} from "vite";
export default configure({{ plugins: [{plugin}] }});"#
    )
}

#[test]
fn imported() {
    assert_eq!(
        Activation::read(&config(
            "crest({ login: false, brand: { product: 'plumb', origin: 'https://plumb.perish.uk' } })"
        )),
        active(
            serde_json::json!({"login":false,"brand":{"product":"plumb","origin":"https://plumb.perish.uk"}})
        )
    );
    assert_eq!(
        Activation::read(
            "import {design} from '@perishlab/design/vite'; export default {plugins:[design()]};"
        ),
        active(serde_json::json!({}))
    );
}

#[test]
fn absent() {
    for text in [
        "// design(\nexport default { plugins: [] };",
        "export default { plugins: [], text: 'design(' };",
        "import {design} from 'other'; export default {plugins: [design()]};",
        "import type {design} from '@perishlab/design/vite'; export default {plugins: [design()]};",
        "import /*comment*/ type {design} from '@perishlab/design/vite'; export default {plugins: [design()]};",
        "import {type /*comment*/ design} from '@perishlab/design/vite'; export default {plugins: [design()]};",
        "import {type design} from '@perishlab/design/vite'; export default {plugins: [design()]};",
        "import {design} from '@perishlab/design/vite'; export default {plugins: [], other: design()};",
    ] {
        assert_eq!(Activation::read(text), Activation::Missing, "{text}");
    }
}

#[test]
fn unsupported() {
    for text in [
        "export default () => ({plugins:[design()]});",
        "export default makeConfig();",
        "import {design} from '@perishlab/design/vite'; export default {plugins:[...design()]};",
        "import {design} from '@perishlab/design/vite'; export default {...other,plugins:[design()]};",
        "import {design} from '@perishlab/design/vite'; export default {plugins:[design(options)]};",
        "import {design} from '@perishlab/design/vite'; export default {plugins:[design(), design()]};",
        "import {design} from '@perishlab/design/vite'; export default {plugins: condition ? [design()] : []};",
        "const design=()=>({}); export default {plugins:[design()]};",
        "export default {plugins:[]}; modifyConfig();",
        "import {design} from '@perishlab/design/vite'; export default {plugins:[design({brand: {product:'plumb',product:'other'}})]};",
        "import {design} from '@perishlab/design/vite'; export default {plugins:[design({brand: JSON.parse('{}')})]};",
        "export default {plugins: [",
    ] {
        assert!(
            matches!(Activation::read(text), Activation::Unknown(_)),
            "{text}"
        );
    }
}
