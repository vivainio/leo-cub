use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use leo::{LeoDocument, Node, NodeId, Operation, OperationBatch, TreeNode};
use serde_json::json;

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
        ..Default::default()
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
            ..Default::default()
        })
        .unwrap();

    let rendered = document.to_xml().unwrap();
    assert!(rendered.contains(r#"<v t="b"><vh>Child</vh>"#));
    assert_eq!(rendered.matches(r#"<v t="c""#).count(), 1);

    // "b" is written in full only at its first occurrence (under "a"); the
    // second occurrence at the top level is elided to `<v t="b"></v>` in the
    // XML, but since it's a clone of the same node it must reparse with the
    // same child, not empty (see `clone_occurrences_all_retain_their_children`
    // in src/xml.rs).
    let reparsed = LeoDocument::parse(&rendered).unwrap();
    assert_eq!(
        reparsed.outline.roots[0].children[0].children[0].node,
        NodeId::from("c")
    );
    assert_eq!(reparsed.outline.roots[1].children.len(), 1);
    assert_eq!(
        reparsed.outline.roots[1].children[0].node,
        NodeId::from("c")
    );
}

#[test]
fn clone_copies_the_source_occurrences_children_so_it_does_not_diverge_on_creation() {
    // "b" ("Child", under "a") has no children of its own in SAMPLE, so give
    // it one first, then clone "b" to the top level.
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
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            }],
            ..Default::default()
        })
        .unwrap();

    document
        .outline
        .apply(&OperationBatch {
            operations: vec![Operation::Clone {
                parent: None,
                parent_headline: None,
                index: Some(0),
                node: NodeId::from("b"),
            }],
            ..Default::default()
        })
        .unwrap();

    // The new clone occurrence is now the first "b" in outline order (root
    // index 0, ahead of the pre-existing occurrence nested under "a"), so if
    // it didn't carry a copy of "b"'s children, serializing straight from
    // this in-memory state would write it as "b"'s one full definition with
    // no children -- silently losing "c" everywhere, not just locally.
    assert_eq!(document.outline.roots[0].node, NodeId::from("b"));
    assert_eq!(document.outline.roots[0].children.len(), 1);
    assert_eq!(
        document.outline.roots[0].children[0].node,
        NodeId::from("c")
    );

    // Round-tripping through XML confirms both occurrences keep "c".
    let reparsed = LeoDocument::parse(&document.to_xml().unwrap()).unwrap();
    assert_eq!(reparsed.outline.roots[0].node, NodeId::from("b"));
    assert_eq!(
        reparsed.outline.roots[0].children[0].node,
        NodeId::from("c")
    );
    let b_under_a = &reparsed.outline.roots[1].children[0];
    assert_eq!(b_under_a.node, NodeId::from("b"));
    assert_eq!(b_under_a.children[0].node, NodeId::from("c"));
}

#[test]
fn clone_creates_a_missing_parent_headline_path_like_insert_tree_does() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    outline
        .apply(&OperationBatch {
            operations: vec![Operation::Clone {
                parent: None,
                parent_headline: Some("Imports/PRs".into()),
                index: None,
                node: NodeId::from("b"),
            }],
            ..Default::default()
        })
        .unwrap();

    let imports = outline
        .roots
        .iter()
        .find(|p| outline.nodes[&p.node].headline == "Imports")
        .unwrap();
    assert_eq!(imports.children.len(), 1);
    let prs = &imports.children[0];
    assert_eq!(outline.nodes[&prs.node].headline, "PRs");
    assert_eq!(prs.children.len(), 1);
    assert_eq!(prs.children[0].node, NodeId::from("b"));
}

#[test]
fn clone_rejects_both_parent_and_parent_headline() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let result = outline.apply(&OperationBatch {
        operations: vec![Operation::Clone {
            parent: Some(NodeId::from("a")),
            parent_headline: Some("Imports/PRs".into()),
            index: None,
            node: NodeId::from("b"),
        }],
        ..Default::default()
    });
    assert!(result.is_err());
}

#[test]
fn insert_tree_generates_gnxs_with_the_batch_prefix_and_keeps_explicit_ones() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Plan": {
            "_body": "top",
            "Milestone": {
                "_gnx": "explicit.1",
                "_body": "pinned"
            }
        }
    }))
    .unwrap();

    outline
        .apply(&OperationBatch {
            gnx_prefix: "acme".into(),
            operations: vec![Operation::InsertTree {
                parent: None,
                parent_headline: None,
                index: None,
                tree,
            }],
        })
        .unwrap();

    let plan_position = outline
        .roots
        .iter()
        .find(|p| outline.nodes[&p.node].headline == "Plan")
        .unwrap();
    let plan = &outline.nodes[&plan_position.node];
    assert!(plan.id.0.starts_with("acme."));
    assert_eq!(plan.body, "top");

    let milestone_position = &plan_position.children[0];
    assert_eq!(milestone_position.node, NodeId::from("explicit.1"));
    assert_eq!(outline.nodes[&milestone_position.node].body, "pinned");
}

#[test]
fn insert_tree_defaults_to_the_cub_gnx_prefix() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Note": { "_body": "" }
    }))
    .unwrap();

    outline
        .apply(&OperationBatch {
            operations: vec![Operation::InsertTree {
                parent: None,
                parent_headline: None,
                index: None,
                tree,
            }],
            ..Default::default()
        })
        .unwrap();

    let note_position = outline
        .roots
        .iter()
        .find(|p| outline.nodes[&p.node].headline == "Note")
        .unwrap();
    assert!(outline.nodes[&note_position.node].id.0.starts_with("cub."));
}

#[test]
fn insert_tree_creates_a_missing_parent_headline_path_and_reuses_it_next_time() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "First": { "_body": "one" }
    }))
    .unwrap();
    outline
        .apply(&OperationBatch {
            operations: vec![Operation::InsertTree {
                parent: None,
                parent_headline: Some("Imports/PRs".into()),
                index: None,
                tree,
            }],
            ..Default::default()
        })
        .unwrap();

    let imports = outline
        .roots
        .iter()
        .find(|p| outline.nodes[&p.node].headline == "Imports")
        .unwrap();
    assert_eq!(imports.children.len(), 1);
    let prs = &imports.children[0];
    assert_eq!(outline.nodes[&prs.node].headline, "PRs");
    assert_eq!(prs.children.len(), 1);
    assert_eq!(outline.nodes[&prs.children[0].node].headline, "First");

    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Second": { "_body": "two" }
    }))
    .unwrap();
    outline
        .apply(&OperationBatch {
            operations: vec![Operation::InsertTree {
                parent: None,
                parent_headline: Some("Imports/PRs".into()),
                index: None,
                tree,
            }],
            ..Default::default()
        })
        .unwrap();

    // Reuses the same "Imports/PRs" nodes rather than creating duplicates.
    assert_eq!(
        outline
            .roots
            .iter()
            .filter(|p| outline.nodes[&p.node].headline == "Imports")
            .count(),
        1
    );
    let imports = outline
        .roots
        .iter()
        .find(|p| outline.nodes[&p.node].headline == "Imports")
        .unwrap();
    assert_eq!(imports.children.len(), 1);
    assert_eq!(imports.children[0].children.len(), 2);
}

#[test]
fn insert_tree_rejects_both_parent_and_parent_headline() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Note": { "_body": "" }
    }))
    .unwrap();
    let batch = OperationBatch {
        operations: vec![Operation::InsertTree {
            parent: Some(NodeId::from("a")),
            parent_headline: Some("Root".into()),
            index: None,
            tree,
        }],
        ..Default::default()
    };
    assert!(outline.apply(&batch).is_err());
}

#[test]
fn replace_tree_by_headline_swaps_the_subtree_at_the_same_position() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "New Root": { "_body": "fresh" }
    }))
    .unwrap();

    outline
        .apply(&OperationBatch {
            operations: vec![Operation::ReplaceTree {
                node: None,
                headline: Some("Root".into()),
                tree,
            }],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(outline.roots.len(), 2);
    assert_eq!(outline.nodes[&outline.roots[0].node].headline, "New Root");
    assert_ne!(outline.roots[0].node, NodeId::from("a"));
    assert!(!outline.nodes.contains_key(&NodeId::from("a")));
}

#[test]
fn replace_tree_by_node_keeps_the_clone_occurrence_still_referencing_it() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Grandchild": { "_body": "" }
    }))
    .unwrap();

    outline
        .apply(&OperationBatch {
            operations: vec![Operation::ReplaceTree {
                node: Some(NodeId::from("b")),
                headline: None,
                tree,
            }],
            ..Default::default()
        })
        .unwrap();

    // "b" is a clone: only its defining occurrence under "a" is replaced.
    // The bare second root still references the original node "b", keeping
    // it alive with its original headline.
    assert!(outline.nodes.contains_key(&NodeId::from("b")));
    assert_eq!(outline.roots[1].node, NodeId::from("b"));
    assert_eq!(outline.nodes[&NodeId::from("b")].headline, "Child");

    assert_ne!(outline.roots[0].children[0].node, NodeId::from("b"));
    assert_eq!(
        outline.nodes[&outline.roots[0].children[0].node].headline,
        "Grandchild"
    );
}

#[test]
fn replace_tree_requires_exactly_one_target() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let batch = OperationBatch {
        operations: vec![Operation::ReplaceTree {
            node: None,
            headline: None,
            tree: BTreeMap::new(),
        }],
        ..Default::default()
    };
    assert!(outline.apply(&batch).is_err());
}

#[test]
fn merge_tree_updates_matching_body_and_adds_missing_children_without_deleting() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Root": {
            "_body": "updated body",
            "New Child": { "_body": "added" }
        }
    }))
    .unwrap();

    outline
        .apply(&OperationBatch {
            operations: vec![Operation::MergeTree {
                parent: None,
                parent_headline: None,
                tree,
            }],
            ..Default::default()
        })
        .unwrap();

    // Existing root "a" keeps its GNX and gains an updated body.
    assert_eq!(outline.roots.len(), 2);
    assert_eq!(outline.roots[0].node, NodeId::from("a"));
    assert_eq!(outline.nodes[&NodeId::from("a")].body, "updated body");

    // Its existing child "b" is untouched, and "New Child" is added beside it.
    assert_eq!(outline.roots[0].children.len(), 2);
    assert!(outline.nodes.contains_key(&NodeId::from("b")));
    assert_eq!(outline.nodes[&NodeId::from("b")].body, "child");
    assert!(
        outline.roots[0]
            .children
            .iter()
            .any(|p| outline.nodes[&p.node].headline == "New Child")
    );
}

#[test]
fn merge_tree_leaves_body_unchanged_when_not_given() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Root": {}
    }))
    .unwrap();

    outline
        .apply(&OperationBatch {
            operations: vec![Operation::MergeTree {
                parent: None,
                parent_headline: None,
                tree,
            }],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(outline.nodes[&NodeId::from("a")].body, "body & text");
}

#[test]
fn merge_tree_creates_a_missing_parent_headline_path() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Note": { "_body": "" }
    }))
    .unwrap();

    outline
        .apply(&OperationBatch {
            operations: vec![Operation::MergeTree {
                parent: None,
                parent_headline: Some("Imports".into()),
                tree,
            }],
            ..Default::default()
        })
        .unwrap();

    let imports = outline
        .roots
        .iter()
        .find(|p| outline.nodes[&p.node].headline == "Imports")
        .unwrap();
    assert_eq!(imports.children.len(), 1);
    assert_eq!(outline.nodes[&imports.children[0].node].headline, "Note");
}

#[test]
fn merge_tree_rejects_an_ambiguous_headline() {
    let mut outline = LeoDocument::parse(SAMPLE).unwrap().outline;
    outline
        .apply(&OperationBatch {
            operations: vec![Operation::Insert {
                parent: None,
                index: None,
                node: Node {
                    id: NodeId::from("c"),
                    headline: "Root".into(),
                    body: String::new(),
                    vnode_attributes: HashMap::new(),
                    tnode_attributes: HashMap::new(),
                },
            }],
            ..Default::default()
        })
        .unwrap();

    let tree: BTreeMap<String, TreeNode> = serde_json::from_value(json!({
        "Root": { "_body": "x" }
    }))
    .unwrap();
    let batch = OperationBatch {
        operations: vec![Operation::MergeTree {
            parent: None,
            parent_headline: None,
            tree,
        }],
        ..Default::default()
    };
    assert!(outline.apply(&batch).is_err());
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
