use leo::{LeoDocument, NodeId, PositionId, RelativeFile, SentinelError};

const PYTHON: &str = r#"# preamble
#@+leo-ver=cub-1-thin
#@0 [root.1] @f example.py
root line
#@+others
#@> first child
def first():
    pass
#@ second child
#@ [child.3] third child
pass
#@-others
root tail
#@-leo
"#;

const NESTED: &str = "#@+leo-ver=cub-1-thin\n#@0 [r] @f x.py\n#@+others\n#@> a\n#@+others\n#@> b\n#@-others\n#@< c\n#@-others\n#@-leo\n";

const BLOCK: &str = r#"/*@+leo-ver=cub-1-thin*/
/*@0 [block.1] @f example.c*/
/*@@language c*/
/*@> child*/
int value;
/*@-leo*/
"#;

#[test]
fn reconstructs_hierarchy_with_relative_depth_and_optional_gnx() {
    let parsed = RelativeFile::parse(PYTHON).unwrap();
    assert_eq!(parsed.root, NodeId::from("root.1"));
    // "first child" (@>) and "second child"/"third child" (@, same depth)
    // are all siblings one level below the root.
    assert_eq!(parsed.outline.roots[0].children.len(), 3);
    let headlines: Vec<_> = parsed.outline.roots[0]
        .children
        .iter()
        .map(|child| parsed.outline.nodes[&child.node].headline.clone())
        .collect();
    assert_eq!(
        headlines,
        vec!["first child", "second child", "third child"]
    );
    assert_eq!(
        parsed.outline.roots[0].children[2].node,
        NodeId::from("child.3")
    );
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("root.1")].body,
        "root line\n@others\nroot tail\n"
    );
}

#[test]
fn steps_back_up_a_level_with_the_shallower_token() {
    let parsed = RelativeFile::parse(NESTED).unwrap();
    assert_eq!(parsed.root, NodeId::from("r"));
    let root_children = &parsed.outline.roots[0].children;
    assert_eq!(root_children.len(), 2);
    let headlines: Vec<_> = root_children
        .iter()
        .map(|child| parsed.outline.nodes[&child.node].headline.clone())
        .collect();
    assert_eq!(headlines, vec!["a", "c"]);
    assert_eq!(root_children[0].children.len(), 1);
    assert_eq!(
        parsed.outline.nodes[&root_children[0].children[0].node].headline,
        "b"
    );
}

#[test]
fn supports_block_comment_sentinels_and_directives() {
    let parsed = RelativeFile::parse(BLOCK).unwrap();
    assert_eq!(parsed.start_delimiter, "/*");
    assert_eq!(parsed.end_delimiter, "*/");
    assert_eq!(
        parsed.outline.nodes[&NodeId::from("block.1")].body,
        "@language c\n"
    );
}

#[test]
fn root_sentinel_requires_an_explicit_gnx() {
    let source = "#@+leo-ver=cub-1-thin\n#@0 @f example.py\n#@-leo\n";
    let error = RelativeFile::parse(source).unwrap_err();
    assert!(matches!(error, SentinelError::MalformedNode { .. }));
}

#[test]
fn relative_depth_cannot_underflow_below_the_root() {
    let source = "#@+leo-ver=cub-1-thin\n#@0 [r] @f example.py\n#@> child\n#@<2 too far\n#@-leo\n";
    let error = RelativeFile::parse(source).unwrap_err();
    assert!(matches!(error, SentinelError::DepthUnderflow { .. }));
}

#[test]
fn merges_only_when_root_identity_matches() {
    let xml = r#"<leo_file><vnodes><v t="root.1"><vh>@f example.py</vh></v></vnodes><tnodes><t tx="root.1"></t></tnodes></leo_file>"#;
    let mut document = LeoDocument::parse(xml).unwrap();
    let parsed = RelativeFile::parse(PYTHON).unwrap();
    parsed
        .merge_into(&mut document.outline, &PositionId("0".into()))
        .unwrap();
    assert_eq!(document.outline.roots[0].children.len(), 3);

    let error = parsed
        .merge_into(&mut document.outline, &PositionId("0/0".into()))
        .unwrap_err();
    assert!(matches!(error, SentinelError::RootMismatch { .. }));
}

#[test]
fn anonymous_nodes_reconcile_to_the_existing_gnx_at_the_same_position() {
    // "first child" and "second child" have no [gnx] in PYTHON; on a second
    // sync they must keep whatever gnx already occupies that structural
    // position, rather than getting a fresh synthetic id every time.
    let xml = r#"<leo_file><vnodes><v t="root.1"><vh>@f example.py</vh>
        <v t="stable-first"><vh>old first</vh></v>
        <v t="stable-second"><vh>old second</vh></v>
    </v></vnodes><tnodes><t tx="root.1"></t><t tx="stable-first">old</t><t tx="stable-second">old</t></tnodes></leo_file>"#;
    let mut document = LeoDocument::parse(xml).unwrap();
    let parsed = RelativeFile::parse(PYTHON).unwrap();
    parsed
        .merge_into(&mut document.outline, &PositionId("0".into()))
        .unwrap();

    let children = &document.outline.roots[0].children;
    assert_eq!(children[0].node, NodeId::from("stable-first"));
    assert_eq!(children[1].node, NodeId::from("stable-second"));
    // The bracketed node keeps its own explicit gnx regardless of position.
    assert_eq!(children[2].node, NodeId::from("child.3"));
    assert_eq!(
        document.outline.nodes[&NodeId::from("stable-first")].headline,
        "first child"
    );
    assert_eq!(
        document.outline.nodes[&NodeId::from("stable-first")].body,
        "def first():\n    pass\n"
    );
}

#[test]
fn anonymous_nodes_with_no_prior_occupant_keep_a_synthetic_id() {
    let xml = r#"<leo_file><vnodes><v t="root.1"><vh>@f example.py</vh></v></vnodes><tnodes><t tx="root.1"></t></tnodes></leo_file>"#;
    let mut document = LeoDocument::parse(xml).unwrap();
    let parsed = RelativeFile::parse(PYTHON).unwrap();
    parsed
        .merge_into(&mut document.outline, &PositionId("0".into()))
        .unwrap();

    let children = &document.outline.roots[0].children;
    assert_ne!(children[0].node, NodeId::from("stable-first"));
    assert_eq!(
        document.outline.nodes[&children[0].node].headline,
        "first child"
    );
}

#[test]
fn merge_preserves_existing_attributes_for_matched_anonymous_nodes() {
    let xml = r#"<leo_file><vnodes><v t="root.1"><vh>@f example.py</vh>
        <v t="stable-first" custom="v"><vh>old first</vh></v>
        <v t="stable-second"><vh>old second</vh></v>
    </v></vnodes><tnodes><t tx="root.1"></t><t tx="stable-first" custom="t">old</t><t tx="stable-second">old</t></tnodes></leo_file>"#;
    let mut document = LeoDocument::parse(xml).unwrap();
    let parsed = RelativeFile::parse(PYTHON).unwrap();
    parsed
        .merge_into(&mut document.outline, &PositionId("0".into()))
        .unwrap();

    let child = &document.outline.nodes[&NodeId::from("stable-first")];
    assert_eq!(
        child.vnode_attributes.get("custom").map(String::as_str),
        Some("v")
    );
    assert_eq!(
        child.tnode_attributes.get("custom").map(String::as_str),
        Some("t")
    );
}
