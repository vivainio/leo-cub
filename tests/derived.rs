use leo::{DerivedFile, LeoDocument, NodeId, PositionId, SentinelError};

const PYTHON: &str = r#"# preamble
#@+leo-ver=5-thin
#@+node:root.1: * @file example.py
root line
#@+others
#@+node:child.1: ** first child
def first():
    pass
#@+node:child.2: ** second child
#@verbatim
#@+node:not-a-sentinel: ** literal
#@-others
root tail
#@-leo
"#;

const BLOCK: &str = r#"/*@+leo-ver=5-thin*/
/*@+node:block.1: * @file example.c*/
/*@@language c*/
/*@+node:block.2: ** child*/
int value;
/*@-leo*/
"#;

const FIRST_LAST: &str = "#!/usr/bin/env python\n#@+leo-ver=5-thin-encoding=utf-8,.\n#@+node:first.1: * @file script.py\n#@@first\nbody\n#@@last\n#@-leo\ntrailer\n";

const INDENTED_OTHERS: &str = r#"#@+leo-ver=5-thin
#@+node:indent.1: * @file nested.py
def outer():
    #@+others
    #@+node:indent.2: ** child
    return 1
    #@-others
#@-leo
"#;

#[test]
fn reconstructs_hierarchy_bodies_and_verbatim_lines() {
    let parsed = DerivedFile::parse(PYTHON).unwrap();
    assert_eq!(parsed.root, NodeId::from("root.1"));
    assert_eq!(parsed.outline.roots[0].children.len(), 2);
    assert_eq!(parsed.locations[&PositionId("0/0".into())], 6);
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("root.1")].body,
        "root line\n@others\nroot tail\n"
    );
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("child.1")].body,
        "def first():\n    pass\n"
    );
    assert!(
        parsed.outline.nodes[&NodeId::from("child.2")]
            .body
            .contains("#@+node:not-a-sentinel")
    );
}

#[test]
fn supports_block_comment_sentinels_and_directives() {
    let parsed = DerivedFile::parse(BLOCK).unwrap();
    assert_eq!(parsed.start_delimiter, "/*");
    assert_eq!(parsed.end_delimiter, "*/");
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("block.1")].body,
        "@language c\n"
    );
}

#[test]
fn restores_first_last_and_encoded_headers() {
    let parsed = DerivedFile::parse(FIRST_LAST).unwrap();
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("first.1")].body,
        "@first #!/usr/bin/env python\nbody\n@last trailer\n"
    );
}

#[test]
fn preserves_directive_and_child_body_indentation() {
    let parsed = DerivedFile::parse(INDENTED_OTHERS).unwrap();
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("indent.1")].body,
        "def outer():\n    @others\n"
    );
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("indent.2")].body,
        "return 1\n"
    );
}

#[test]
fn preserves_expansion_indentation_relative_to_nested_node() {
    let source = "#@+leo-ver=5-thin\n#@+node:r: * root\n#@+others\n    #@+node:c: ** child\n        #@+others\n        #@+node:g: *3* grandchild\n        body\n        #@-others\n    #@-others\n#@-leo\n";
    let parsed = DerivedFile::parse(source).unwrap();
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("c")].body,
        "    @others\n"
    );
}

#[test]
fn merges_only_when_root_identity_matches() {
    let xml = r#"<leo_file><vnodes><v t="root.1"><vh>@file example.py</vh></v></vnodes><tnodes><t tx="root.1"></t></tnodes></leo_file>"#;
    let mut document = LeoDocument::parse(xml).unwrap();
    let parsed = DerivedFile::parse(PYTHON).unwrap();
    parsed
        .merge_into(&mut document.outline, &PositionId("0".into()))
        .unwrap();
    assert_eq!(document.outline.roots[0].children.len(), 2);

    let error = parsed
        .merge_into(&mut document.outline, &PositionId("0/0".into()))
        .unwrap_err();
    assert!(matches!(error, SentinelError::RootMismatch { .. }));
}

#[test]
fn merge_preserves_existing_node_attributes() {
    let parsed = DerivedFile::parse(PYTHON).unwrap();
    let mut document = LeoDocument::parse(
        r#"<leo_file><vnodes><v t="root.1"><vh>root</vh><v t="child.1" custom="v"><vh>old</vh></v></v></vnodes><tnodes><t tx="root.1"></t><t tx="child.1" custom="t">old</t></tnodes></leo_file>"#,
    )
    .unwrap();

    parsed
        .merge_into(&mut document.outline, &PositionId("0".into()))
        .unwrap();

    let child = &document.outline.nodes[&NodeId::from("child.1")];
    assert_eq!(
        child.vnode_attributes.get("custom").map(String::as_str),
        Some("v")
    );
    assert_eq!(
        child.tnode_attributes.get("custom").map(String::as_str),
        Some("t")
    );
}
