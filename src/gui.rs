//! Native GPUI desktop frontend (`cub gui FILE`).
//!
//! Sits next to the TUI and shares the `leo` library for everything that
//! touches the document: loading, derived/external file sync, and saving.
//! MVP scope: outline tree, headline + body editing, save.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::Result;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::input::{InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::{TreeItem, TreeState, tree};
use gpui_kit::*;
use leo::{
    LeoDocument, Node, NodeId, Operation, OperationBatch, OriginalExternalState, Outline, Position,
    PositionId, WritableExternalFile, load_derived_files, save_document,
};

actions!(
    leo_gui,
    [Save, Quit, InsertSibling, InsertChild, DeleteNode]
);

struct GuiApp {
    document: LeoDocument,
    path: PathBuf,
    writable_external: HashMap<NodeId, WritableExternalFile>,
    original_external: OriginalExternalState,
    dirty: bool,
    status: String,
    quit_armed: bool,
    /// Nodes the user has expanded (by node, not position, so the state
    /// survives inserts and deletes that shift position paths).
    expanded: HashSet<NodeId>,
    selected: Option<PositionId>,
    tree: Entity<TreeState>,
    headline: Entity<InputState>,
    body: Entity<TextareaState>,
    _subscriptions: Vec<Subscription>,
}

fn build_items(outline: &Outline, expanded: &HashSet<NodeId>) -> Vec<TreeItem> {
    fn build(
        outline: &Outline,
        position: &Position,
        path: String,
        expanded: &HashSet<NodeId>,
    ) -> TreeItem {
        let headline = outline
            .nodes
            .get(&position.node)
            .map(|node| node.headline.as_str())
            .unwrap_or("<missing>");
        let children = position
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| build(outline, child, format!("{path}/{index}"), expanded))
            .collect::<Vec<_>>();
        TreeItem::new(path.clone(), headline.to_owned())
            .expanded(expanded.contains(&position.node))
            .children(children)
    }
    outline
        .roots
        .iter()
        .enumerate()
        .map(|(index, root)| build(outline, root, index.to_string(), expanded))
        .collect()
}

impl GuiApp {
    fn new(
        document: LeoDocument,
        path: PathBuf,
        writable_external: HashMap<NodeId, WritableExternalFile>,
        original_external: OriginalExternalState,
        status: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let expanded = HashSet::new();
        let items = build_items(&document.outline, &expanded);
        let tree = cx.new(|cx| TreeState::new(cx).items(items));
        let headline = cx.new(|cx| InputState::new(window, cx));
        let body = cx.new(|cx| TextareaState::new(window, cx));

        let mut subscriptions = Vec::new();
        // The tree has no selection event; observe it and diff the index.
        subscriptions.push(cx.observe_in(&tree, window, |this, _, window, cx| {
            this.sync_selection(window, cx);
        }));
        subscriptions.push(cx.subscribe(&tree, |this, _, event, _| {
            use gpui_kit::component::tree::TreeEvent;
            let (id, open) = match event {
                TreeEvent::Expanded(id) => (id, true),
                TreeEvent::Collapsed(id) => (id, false),
            };
            let node = this
                .document
                .outline
                .position(&PositionId(id.to_string()))
                .map(|position| position.node.clone());
            if let Some(node) = node {
                if open {
                    this.expanded.insert(node);
                } else {
                    this.expanded.remove(&node);
                }
            }
        }));
        subscriptions.push(
            cx.subscribe(&headline, |this, state, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = state.read(cx).value().to_string();
                    if this.edit_selected(cx, |node| &mut node.headline, value) {
                        let keep = this.selected.clone();
                        this.refresh_tree(keep, cx);
                    }
                }
            }),
        );
        subscriptions.push(cx.subscribe(&body, |this, state, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = state.read(cx).value().to_string();
                this.edit_selected(cx, |node| &mut node.body, value);
            }
        }));

        Self {
            document,
            path,
            writable_external,
            original_external,
            dirty: false,
            quit_armed: false,
            status,
            expanded,
            selected: None,
            tree,
            headline,
            body,
            _subscriptions: subscriptions,
        }
    }

    fn sync_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self
            .tree
            .read(cx)
            .selected_item()
            .map(|item| PositionId(item.id.to_string()));
        if id == self.selected {
            return;
        }
        self.selected = id.clone();
        let node = id
            .and_then(|id| self.document.outline.position(&id).map(|p| p.node.clone()))
            .and_then(|node_id| self.document.outline.nodes.get(&node_id).cloned());
        let (headline, body) = node
            .map(|node| (node.headline, node.body))
            .unwrap_or_default();
        self.headline
            .update(cx, |state, cx| state.set_value(headline, window, cx));
        self.body
            .update(cx, |state, cx| state.set_value(body, window, cx));
        cx.notify();
    }

    /// Writes `value` into the selected node's field. Headline edits also
    /// refresh the tree label (rebuilt lazily, preserving expansion).
    fn edit_selected(
        &mut self,
        cx: &mut Context<Self>,
        field: impl Fn(&mut leo::Node) -> &mut String,
        value: String,
    ) -> bool {
        let Some(selected) = self.selected.clone() else {
            return false;
        };
        let Some(node_id) = self
            .document
            .outline
            .position(&selected)
            .map(|p| p.node.clone())
        else {
            return false;
        };
        let Some(node) = self.document.outline.nodes.get_mut(&node_id) else {
            return false;
        };
        let slot = field(node);
        if *slot == value {
            return false;
        }
        *slot = value;
        self.dirty = true;
        cx.notify();
        true
    }

    /// Rebuilds the tree rows keeping the expansion state, and selects
    /// `select`. For a headline edit pass the current selection: it equals
    /// `self.selected`, so the selection observer sees no change and does
    /// not reset the editors mid-typing. After a structural edit pass the
    /// new selection (or `None`), which the observer then loads.
    fn refresh_tree(&mut self, select: Option<PositionId>, cx: &mut Context<Self>) {
        let items = build_items(&self.document.outline, &self.expanded);
        self.tree.update(cx, |state, cx| {
            state.set_items(items, cx);
            if let Some(id) = select {
                let id: SharedString = id.0.into();
                if let Some(ix) = state.index_of(&id) {
                    state.set_selected_index(Some(ix), cx);
                }
            }
        });
    }

    /// Applies one library [`Operation`] (validated, all-or-nothing) and
    /// refreshes the tree. Returns whether it applied.
    fn apply_op(
        &mut self,
        op: Operation,
        select: Option<PositionId>,
        cx: &mut Context<Self>,
    ) -> bool {
        let batch = OperationBatch {
            operations: vec![op],
            ..OperationBatch::default()
        };
        match self.document.outline.apply(&batch) {
            Ok(_) => {
                self.dirty = true;
                self.quit_armed = false;
                self.refresh_tree(select, cx);
                cx.notify();
                true
            }
            Err(error) => {
                self.status = format!("edit failed: {error}");
                cx.notify();
                false
            }
        }
    }

    fn selected_node(&self) -> Option<NodeId> {
        let id = self.selected.as_ref()?;
        Some(self.document.outline.position(id)?.node.clone())
    }

    fn new_node() -> Node {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let id = NodeId(format!("cub.{}.{}", now.as_secs(), now.subsec_nanos()));
        Node {
            id,
            headline: "New Headline".into(),
            body: String::new(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        }
    }

    fn insert_sibling(&mut self, _: &InsertSibling, window: &mut Window, cx: &mut Context<Self>) {
        let node = Self::new_node();
        let (parent, index, select) = match &self.selected {
            Some(selected) => {
                let Some((parent_path, index)) = split_path(selected) else {
                    return;
                };
                let parent = parent_path
                    .as_ref()
                    .and_then(|p| self.document.outline.position(p))
                    .map(|p| p.node.clone());
                let select = join_path(parent_path.as_ref(), index + 1);
                (parent, Some(index + 1), select)
            }
            None => (
                None,
                None,
                join_path(None, self.document.outline.roots.len()),
            ),
        };
        if self.apply_op(
            Operation::Insert {
                parent,
                index,
                node,
            },
            Some(select),
            cx,
        ) {
            self.focus_headline(window, cx);
        }
    }

    fn insert_child(&mut self, _: &InsertChild, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(selected), Some(parent)) = (self.selected.clone(), self.selected_node()) else {
            return;
        };
        let count = self
            .document
            .outline
            .position(&selected)
            .map_or(0, |p| p.children.len());
        self.expanded.insert(parent.clone());
        let select = join_path(Some(&selected), count);
        let op = Operation::Insert {
            parent: Some(parent),
            index: None,
            node: Self::new_node(),
        };
        if self.apply_op(op, Some(select), cx) {
            self.focus_headline(window, cx);
        }
    }

    fn delete_node(&mut self, _: &DeleteNode, _: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.selected.clone() else {
            return;
        };
        let headline = self
            .selected_node()
            .and_then(|id| self.document.outline.nodes.get(&id))
            .map(|node| node.headline.clone())
            .unwrap_or_default();
        if self.apply_op(Operation::Remove { position: selected }, None, cx) {
            self.status = format!("deleted \"{headline}\"");
        }
    }

    fn focus_headline(&self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.headline.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    }

    fn save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        match save_document(
            &self.document,
            &self.path,
            &mut self.writable_external,
            &self.original_external,
        ) {
            Ok(()) => {
                self.dirty = false;
                self.quit_armed = false;
                self.status = format!("saved {}", self.path.display());
            }
            Err(error) => self.status = format!("save failed: {error}"),
        }
        cx.notify();
    }

    fn quit(&mut self, _: &Quit, _: &mut Window, cx: &mut Context<Self>) {
        if self.dirty && !self.quit_armed {
            self.status = "unsaved changes -- Cmd-S to save, Cmd-Q again to discard".into();
            self.quit_armed = true;
            cx.notify();
        } else {
            cx.quit();
        }
    }
}

impl Render for GuiApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let title = format!(
            "{}{}",
            self.path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            if self.dirty { " *" } else { "" }
        );
        div()
            .key_context("LeoGui")
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::insert_sibling))
            .on_action(cx.listener(Self::insert_child))
            .on_action(cx.listener(Self::delete_node))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .w(px(340.))
                            .h_full()
                            .border_r_1()
                            .border_color(theme.border)
                            .child(tree(&self.tree, |ix, entry, selected, _, _| {
                                let item = entry.item();
                                ListItem::new(ix)
                                    .selected(selected)
                                    .pl(px(8. + 16. * entry.depth() as f32))
                                    .child(format!(
                                        "{} {}",
                                        if entry.is_folder() {
                                            if entry.is_expanded() { "▾" } else { "▸" }
                                        } else {
                                            "·"
                                        },
                                        item.label
                                    ))
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_2()
                            .child(gpui_kit::component::input::Input::new(&self.headline))
                            .child(div().flex_1().min_h_0().child(Textarea::new(&self.body))),
                    ),
            )
            .child(
                div()
                    .h(px(24.))
                    .px_2()
                    .flex()
                    .items_center()
                    .border_t_1()
                    .border_color(theme.border)
                    .text_sm()
                    .child(format!("{title}  —  {}", self.status)),
            )
    }
}

pub fn run(path: PathBuf, load_derived: bool) -> Result<()> {
    let mut document = LeoDocument::open(&path)?;
    let (status, writable_external, original_external) = if load_derived {
        let report = load_derived_files(&mut document.outline, &path);
        let status = if report.errors.is_empty() {
            format!("loaded {} derived file(s)", report.loaded)
        } else {
            format!(
                "loaded {}; {} error(s): {}",
                report.loaded,
                report.errors.len(),
                report.errors.join(" | ")
            )
        };
        (
            status,
            report.writable_external,
            OriginalExternalState {
                children: report.original_children,
                bodies: report.original_bodies,
                nodes: report.original_nodes,
            },
        )
    } else {
        (
            "derived files disabled".to_owned(),
            HashMap::new(),
            OriginalExternalState::default(),
        )
    };

    let mut seed = Some((document, path, writable_external, original_external, status));
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        cx.bind_keys([
            KeyBinding::new("cmd-s", Save, Some("LeoGui")),
            KeyBinding::new("cmd-q", Quit, Some("LeoGui")),
            KeyBinding::new("cmd-i", InsertSibling, Some("LeoGui")),
            KeyBinding::new("cmd-shift-i", InsertChild, Some("LeoGui")),
            KeyBinding::new("cmd-backspace", DeleteNode, Some("LeoGui")),
        ]);
        let (document, path, writable, original, status) = seed.take().expect("run once");
        gpui_kit::open_window(WindowOptions::default(), cx, move |window, cx| {
            cx.new(|cx| GuiApp::new(document, path, writable, original, status, window, cx))
        })
        .expect("failed to open window");
        cx.activate(true);
    });
    Ok(())
}

/// Splits `a/b/c` into (`Some(a/b)`, 2); a root `3` into (`None`, 3).
fn split_path(id: &PositionId) -> Option<(Option<PositionId>, usize)> {
    match id.0.rsplit_once('/') {
        Some((parent, last)) => Some((Some(PositionId(parent.to_owned())), last.parse().ok()?)),
        None => Some((None, id.0.parse().ok()?)),
    }
}

fn join_path(parent: Option<&PositionId>, index: usize) -> PositionId {
    match parent {
        Some(parent) => PositionId(format!("{}/{index}", parent.0)),
        None => PositionId(index.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn paths_split_and_join() {
        let nested = PositionId("2/0/5".into());
        let (parent, index) = split_path(&nested).unwrap();
        assert_eq!(parent, Some(PositionId("2/0".into())));
        assert_eq!(index, 5);
        assert_eq!(join_path(parent.as_ref(), index), nested);
        assert_eq!(split_path(&PositionId("3".into())), Some((None, 3)));
        assert_eq!(join_path(None, 3), PositionId("3".into()));
        assert_eq!(split_path(&PositionId("x".into())), None);
    }

    fn node(id: &str) -> Node {
        Node {
            id: NodeId(id.into()),
            headline: id.into(),
            body: String::new(),
            vnode_attributes: HashMap::new(),
            tnode_attributes: HashMap::new(),
        }
    }

    fn leaf(id: &str) -> Position {
        Position {
            node: NodeId(id.into()),
            children: vec![],
        }
    }

    fn outline() -> Outline {
        let mut outline = Outline::default();
        for id in ["a", "b", "c"] {
            outline.nodes.insert(NodeId(id.into()), node(id));
        }
        outline.roots = vec![
            Position {
                node: NodeId("a".into()),
                children: vec![leaf("b")],
            },
            leaf("c"),
        ];
        outline
    }

    #[::core::prelude::v1::test]
    fn expanded_tree_items_follow_nodes_not_paths() {
        let expanded: HashSet<NodeId> = [NodeId("a".into())].into();
        let items = build_items(&outline(), &expanded);
        assert!(items[0].is_expanded());
        assert!(!items[1].is_expanded());
        assert_eq!(items[0].children[0].id.as_ref(), "0/0");
    }

    #[::core::prelude::v1::test]
    fn insert_and_remove_operations_keep_the_outline_valid() {
        let mut outline = outline();
        let inserted = GuiApp::new_node();
        let batch = OperationBatch {
            operations: vec![Operation::Insert {
                parent: Some(NodeId("a".into())),
                index: None,
                node: inserted,
            }],
            ..OperationBatch::default()
        };
        outline.apply(&batch).unwrap();
        assert_eq!(outline.roots[0].children.len(), 2);
        let batch = OperationBatch {
            operations: vec![Operation::Remove {
                position: PositionId("0/1".into()),
            }],
            ..OperationBatch::default()
        };
        outline.apply(&batch).unwrap();
        assert_eq!(outline.roots[0].children.len(), 1);
        assert!(outline.validate().is_empty());
    }
}
