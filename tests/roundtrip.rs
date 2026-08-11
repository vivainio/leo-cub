use leo::{LeoDocument, NodeId, Operation, OperationBatch};

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
