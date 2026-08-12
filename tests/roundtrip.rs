use std::path::PathBuf;

use leo::{LeoDocument, Node, NodeId, Operation, OperationBatch};
use std::collections::HashMap;

const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<leo_file><leo_header file_format="2"/><globals custom="keep"/><vnodes>
<v t="a"><vh>Root</vh><v t="b"><vh>Child</vh></v></v><v t="b"></v>
</vnodes><tnodes><t tx="a">body &amp; text</t><t tx="b">child</t></tnodes></leo_file>"#;

#[test]
fn parses_clones_and_preserves_envelope() {
    let doc = LeoDocument::parse(SAMPLE).unwrap();
    assert_eq!(doc.outline.roots[0].children[0].node, NodeId::from("b"));
    assert_eq!(doc.outline.roots[1].node, NodeId::from("b"));
    assert_eq!(doc.outline.nodes[&NodeId::from("a")].body, "body & text");
    assert!(doc.to_xml().unwrap().contains("<globals custom=\"keep\"/>"));
}

#[test]
fn batch_is_atomic_on_failed_precondition() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let before = outline.clone();
    let batch = OperationBatch {
        operations: vec![Operation::SetBody {
            node: NodeId::from("a"),
            body: "new".into(),
            expected: Some("wrong".into()),
        }],
    };
    assert!(outline.apply(&batch).is_err());
    assert_eq!(outline, before);
}

#[test]
fn insert_uses_parent_gnx_and_updates_the_shared_clone_subtree() {
    let mut document = LeoDocument::parse(SAMPLE).unwrap();
    document
        .outline
        .apply(&OperationBatch {
            operations: vec![Operation::Insert {
                parent: Some(NodeId::from("b")),
                index: None,
                node: Node {
                    id: NodeId::from("c"),
                    headline: "Grandchild".into(),
                    body: "new body".into(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            }],
        })
        .unwrap();

    let rendered = document.to_xml().unwrap();
    assert!(rendered.contains(r#"<v t="b"><vh>Child</vh>"#));
    assert_eq!(rendered.matches(r#"<v t="c""#).count(), 1);

    let reparsed = LeoDocument::parse(&rendered).unwrap();
    assert_eq!(
        reparsed.outline.roots[0].children[0].children[0].node,
        NodeId::from("c")
    );
    assert!(reparsed.outline.roots[1].children.is_empty());
}

#[test]
fn project_outline_parses_validates_and_round_trips() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("leo-cub.leo");
    let document = LeoDocument::open(path).unwrap();

    assert!(document.outline.validate().is_empty());
    assert!(!document.outline.nodes.is_empty());
    assert!(!document.outline.roots.is_empty());

    let rendered = document.to_xml().unwrap();
    assert!(rendered.contains("<leo_header"));
    assert!(rendered.contains("<globals"));
    assert!(rendered.contains("<preferences"));
    assert!(rendered.contains("<find_panel_settings"));

    let reparsed = LeoDocument::parse(&rendered).unwrap();
    assert_eq!(reparsed.outline, document.outline);
    assert!(reparsed.outline.validate().is_empty());
}
