use super::read;

fn source(body: &str) -> String {
    format!(r#"<script>import {{Hero as Brand}} from '@perishlab/design';</script>{body}"#)
}

#[test]
fn unrelated() {
    for body in [
        r#"{#each items as item}<p>{item}</p>{/each}"#,
        r#"{#each items as {value}, index (value)}<p>{index}</p>{/each}"#,
        r#"{#each items as Brand}<p>{Brand}</p>{/each}"#,
        r#"{#snippet render(Brand)}<p>{Brand}</p>{/snippet}"#,
        r#"{#await promise}<p>Waiting</p>{:then Brand}<p>{Brand}</p>{:catch Brand}<p>{Brand}</p>{/await}"#,
        r#"{#await promise then Brand}<p>{Brand}</p>{/await}"#,
        r#"{#if ready}{@const Brand = value}<p>{Brand}</p>{/if}"#,
        r#"<Other let:Brand><p>{Brand}</p></Other>"#,
    ] {
        let text = source(&format!(r#"<Brand mark="plumb"/>{body}"#));
        let marks = read(&text).unwrap();
        assert_eq!(marks.len(), 1, "{body}");
        assert_eq!(marks[0].product, "plumb");
    }
}

#[test]
fn business() {
    for body in [
        r#"{#each items as Hero}<Hero mark="ordinary-business"/>{/each}"#,
        r#"{#snippet render(Hero)}<Hero mark="ordinary-business"/>{/snippet}"#,
        r#"{#await promise then Hero}<Hero mark="ordinary-business"/>{/await}"#,
        r#"{#if ready}{@const Hero = value}<Hero mark="ordinary-business"/>{/if}"#,
    ] {
        let text = format!(r#"<script>import {{Hero}} from 'business';</script>{body}"#);
        assert!(read(&text).unwrap().is_empty(), "{body}");
    }
}

#[test]
fn shadowed() {
    for body in [
        r#"{#each things as Brand}<Brand mark="plumb"/>{/each}"#,
        r#"{#each things as {view: Brand}}<Brand mark="plumb"/>{/each}"#,
        r#"{#snippet render(Brand)}<Brand mark="plumb"/>{/snippet}"#,
        r#"{#await promise then Brand}<Brand mark="plumb"/>{/await}"#,
        r#"{#await promise}<p>Waiting</p>{:then Brand}<Brand mark="plumb"/>{/await}"#,
        r#"{#await promise}<p>Waiting</p>{:catch Brand}<Brand mark="plumb"/>{/await}"#,
        r#"{#if ready}{@const Brand = other}<Brand mark="plumb"/>{/if}"#,
        r#"<Other let:Brand><Brand mark="plumb"/></Other>"#,
    ] {
        assert!(read(&source(body)).is_err(), "{body}");
    }
}

#[test]
fn hoisted() {
    for body in [
        r#"<Brand mark="plumb"/>{#snippet Brand()}<p>Business</p>{/snippet}"#,
        r#"{#if ready}<Brand mark="plumb"/>{@const Brand = other}{/if}"#,
    ] {
        assert!(read(&source(body)).is_err(), "{body}");
    }
}

#[test]
fn unresolved() {
    for body in [
        r#"{#each things as thing}<Brand mark="plumb"/>{/each}"#,
        r#"{#snippet render(thing)}<Brand mark="plumb"/>{/snippet}"#,
        r#"{#await promise}<Brand mark="plumb"/>{/await}"#,
        r#"{#if ready}{@const value = other}<Brand mark="plumb"/>{/if}"#,
        r#"<Other let:value><Brand mark="plumb"/></Other>"#,
    ] {
        assert!(read(&source(body)).is_err(), "{body}");
    }
}

#[test]
fn dynamic() {
    for body in [
        r#"<Brand mark={unknown}/>{#each things as thing}<p>{thing}</p>{/each}"#,
        r#"<Brand mark="plumb" {...props}/>{#snippet render(item)}<p>{item}</p>{/snippet}"#,
    ] {
        assert!(read(&source(body)).is_err(), "{body}");
    }
}

#[test]
fn generic() {
    for value in [
        "Item",
        "Item extends {text: string}, Value = unknown",
        "Brand",
    ] {
        let text = source(r#"<Brand mark="plumb"/>{#each items as item}<p>{item}</p>{/each}"#)
            .replacen(
                "<script>",
                &format!(r#"<script lang="ts" generics="{value}">"#),
                1,
            );
        assert_eq!(read(&text).unwrap().len(), 1, "{value}");
    }
    for attributes in [
        r#"generics="Item""#,
        r#"lang="js" generics="Item""#,
        r#"lang="ts" generics={dynamic}"#,
        r#"lang="ts" generics="""#,
        r#"lang="ts" generics="Item extends""#,
        r#"lang="ts" generics="Item> = unknown; const extra = 1; type Other<T""#,
        r#"lang="ts" generics="Item> = unknown; //""#,
        r#"lang="ts" lang="ts""#,
        r#"lang="ts" generics="Item" generics="Other""#,
        r#"lang="ts" module generics="Item""#,
    ] {
        let text = source(r#"<Brand mark="plumb"/>"#).replacen(
            "<script>",
            &format!("<script {attributes}>"),
            1,
        );
        assert!(read(&text).is_err(), "{attributes}");
    }
}
