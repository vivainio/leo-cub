//! Native GPUI desktop frontend (`cub gui FILE`).
//!
//! Sits next to the TUI and shares the `leo` library for everything that
//! touches the document: loading, derived/external file sync, and saving.
//! MVP scope: outline tree, headline + body editing, save.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Result;
use gpui_kit::component::highlighter::{LanguageConfig, LanguageRegistry};
use gpui_kit::component::input::{Editor, EditorState, InputEvent, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::tree::{TreeItem, TreeState, tree};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _, Theme, TitleBar,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use leo::{
    LeoDocument, MoveDirection, Node, NodeId, Operation, OperationBatch, OriginalExternalState,
    Outline, Position, PositionId, Subtree, WritableExternalFile, derived_filename,
    external_filename, load_derived_files, save_document,
};

actions!(
    leo_gui,
    [
        Save,
        Quit,
        InsertSibling,
        InsertChild,
        DeleteNode,
        MoveUp,
        MoveDown,
        MoveLeft,
        MoveRight,
        CopyTree,
        CutTree,
        PasteTree,
        PasteClone,
        FindFocus,
        RenameInline,
        CancelRowEdit,
        TreeLeft,
        TreeRight,
        FocusTree,
        MinimizeWindow,
        OpenFile,
        OpenRecent0,
        OpenRecent1,
        OpenRecent2,
        OpenRecent3,
        OpenRecent4,
        OpenRecent5,
        OpenRecent6,
        OpenRecent7,
        OpenRecent8,
        OpenRecent9,
        ZoomWindow,
    ]
);

struct GuiApp {
    document: LeoDocument,
    path: PathBuf,
    writable_external: HashMap<NodeId, WritableExternalFile>,
    original_external: OriginalExternalState,
    /// Nodes produced by loading derived files (`@auto`, `@file`, ...).
    derived_nodes: HashSet<NodeId>,
    clipboard: Option<Subtree>,
    dirty: bool,
    status: String,
    quit_armed: bool,
    /// Nodes the user has expanded (by node, not position, so the state
    /// survives inserts and deletes that shift position paths).
    expanded: HashSet<NodeId>,
    /// Tree rows whose node also appears elsewhere (clones). Cached so the
    /// row renderer needn't walk the outline on every frame.
    clone_paths: Rc<HashSet<String>>,
    selected: Option<PositionId>,
    /// Inline headline edit in the tree row (`h` / F2).
    row_edit: Option<RowEdit>,
    /// Give the tree keyboard focus on the first frame, so the arrow keys
    /// work straight after launch.
    needs_tree_focus: bool,
    /// Whether to load derived (`@file`, `@auto`, ...) files when opening.
    load_derived: bool,
    /// Recently opened outlines, newest first (persisted between runs).
    recent: Vec<PathBuf>,
    /// Started without a file: show the Open dialog on the first frame.
    open_on_start: bool,
    /// Set by the insert commands: once the new node is selected, start
    /// the inline headline edit on it (with its text selected).
    rename_after_select: bool,
    tree: Entity<TreeState>,
    headline: Entity<InputState>,
    search: Entity<InputState>,
    search_matches: Vec<PositionId>,
    search_cursor: usize,
    body: Entity<EditorState>,
    _subscriptions: Vec<Subscription>,
}

struct RowEdit {
    path: String,
    input: Entity<InputState>,
    _subscription: Subscription,
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
        // An empty headline would render as an invisible, hard-to-click row.
        let label = if headline.trim().is_empty() {
            "(empty)"
        } else {
            headline
        };
        TreeItem::new(path.clone(), label.to_owned())
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
        loaded: Loaded,
        load_derived: bool,
        recent: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let Loaded {
            document,
            path,
            writable_external,
            original_external,
            derived_nodes,
            status,
        } = loaded;
        let open_on_start = loaded_is_untitled(&path);
        let expanded = HashSet::new();
        let items = build_items(&document.outline, &expanded);
        let clone_paths = Rc::new(find_clone_paths(&document.outline));
        let tree = cx.new(|cx| TreeState::new(cx).items(items));
        let headline = cx.new(|cx| InputState::new(window, cx).placeholder("Headline"));
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(format!("Find ({}-F)", mod_key())));
        let body = cx.new(|cx| EditorState::new(window, cx).language("text"));

        // Closing the window must not silently drop unsaved edits: the first
        // close warns, a second close discards (same as Cmd-Q).
        let weak = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            weak.update(cx, |this, cx| {
                if this.dirty && !this.quit_armed {
                    this.quit_armed = true;
                    this.status = format!(
                        "unsaved changes -- {}-S to save, close again to discard",
                        mod_key()
                    );
                    cx.notify();
                    false
                } else {
                    true
                }
            })
            .unwrap_or(true)
        });

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
        subscriptions.push(cx.subscribe_in(
            &headline,
            window,
            |this, state, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    let value = state.read(cx).value().to_string();
                    if this.edit_selected(cx, |node| &mut node.headline, value) {
                        let keep = this.selected.clone();
                        this.refresh_tree(keep, cx);
                    }
                }
                // Enter accepts the headline and moves on to the body.
                InputEvent::PressEnter { .. } => {
                    let handle = this.body.read(cx).focus_handle(cx);
                    window.focus(&handle, cx);
                }
                _ => {}
            },
        ));
        subscriptions.push(cx.subscribe(
            &search,
            |this, state, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    let query = state.read(cx).value().to_string();
                    this.search_matches = find_positions(&this.document.outline, &query);
                    this.search_cursor = 0;
                    this.status = if query.is_empty() {
                        String::new()
                    } else {
                        format!("{} match(es)", this.search_matches.len())
                    };
                    cx.notify();
                }
                InputEvent::PressEnter { shift, .. } => {
                    let delta = if *shift { -1 } else { 1 };
                    this.step_search(delta, cx);
                }
                _ => {}
            },
        ));
        subscriptions.push(cx.subscribe(&body, |this, state, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = state.read(cx).value().to_string();
                this.edit_selected(cx, |node| &mut node.body, value);
            }
        }));

        let this = Self {
            document,
            path,
            writable_external,
            original_external,
            derived_nodes,
            clipboard: None,
            dirty: false,
            quit_armed: false,
            status,
            expanded,
            clone_paths,
            selected: None,
            row_edit: None,
            needs_tree_focus: true,
            load_derived,
            recent,
            open_on_start,
            rename_after_select: false,
            tree,
            headline,
            search,
            search_matches: Vec::new(),
            search_cursor: 0,
            body,
            _subscriptions: subscriptions,
        };
        // Open with the first node selected so the editors aren't blank.
        if !this.document.outline.roots.is_empty() {
            this.tree
                .update(cx, |state, cx| state.set_selected_index(Some(0), cx));
        }
        this
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
        let language = self
            .selected
            .as_ref()
            .map_or("text", |id| language_for(&self.document.outline, id));
        let (headline, body) = node
            .map(|node| (node.headline, node.body))
            .unwrap_or_default();
        self.headline
            .update(cx, |state, cx| state.set_value(headline, window, cx));
        self.body.update(cx, |state, cx| {
            state.set_highlighter(language, cx);
            state.set_line_number(language != "text", window, cx);
            state.set_value(body, window, cx);
        });
        if std::mem::take(&mut self.rename_after_select) {
            self.rename_inline(&RenameInline, window, cx);
        }
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
        if self.readonly(&node_id) {
            return false;
        }
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
        self.clone_paths = Rc::new(find_clone_paths(&self.document.outline));
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
                self.structure_changed(select, cx);
                true
            }
            Err(error) => {
                self.status = format!("edit failed: {error}");
                cx.notify();
                false
            }
        }
    }

    fn structure_changed(&mut self, select: Option<PositionId>, cx: &mut Context<Self>) {
        self.dirty = true;
        self.quit_armed = false;
        self.refresh_tree(select, cx);
        cx.notify();
    }

    /// A node loaded from a derived file that isn't writable back.
    fn readonly(&self, node: &NodeId) -> bool {
        self.derived_nodes.contains(node)
            && !self
                .writable_external
                .values()
                .any(|file| file.original.nodes.contains_key(node))
    }

    /// Read-only derived nodes, `@auto`-family roots (regenerated on every
    /// load) and `@edit` roots (flat bodies) can't take new children.
    fn refuses_children(&self, node: &NodeId) -> bool {
        self.readonly(node)
            || self
                .document
                .outline
                .nodes
                .get(node)
                .and_then(|n| derived_filename(&n.headline))
                .is_some_and(|(auto, directive, _)| auto || directive == "@edit")
    }

    fn refuse(&mut self, message: &str, cx: &mut Context<Self>) {
        self.status = message.into();
        cx.notify();
    }

    fn move_selected(&mut self, direction: MoveDirection, cx: &mut Context<Self>) {
        let (Some(selected), Some(node)) = (self.selected.clone(), self.selected_node()) else {
            return;
        };
        if self.readonly(&node) {
            return self.refuse("@auto subtrees cannot be moved", cx);
        }
        let refuses: HashSet<NodeId> = self
            .document
            .outline
            .nodes
            .keys()
            .filter(|id| self.refuses_children(id))
            .cloned()
            .collect();
        match self
            .document
            .outline
            .move_position(&selected, direction, &|id| refuses.contains(id))
        {
            Ok(target) => {
                if direction == MoveDirection::Right
                    && let Some((parent, _)) = target.0.rsplit_once('/')
                    && let Some(parent) = self.document.outline.position(&PositionId(parent.into()))
                {
                    self.expanded.insert(parent.node.clone());
                }
                self.structure_changed(Some(target), cx);
                self.status = "node moved".into();
            }
            Err(error) => self.refuse(&error.to_string(), cx),
        }
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selected(MoveDirection::Up, cx);
    }
    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selected(MoveDirection::Down, cx);
    }
    fn move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selected(MoveDirection::Left, cx);
    }
    fn move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_selected(MoveDirection::Right, cx);
    }

    fn copy_tree(&mut self, _: &CopyTree, _: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.selected.clone() else {
            return;
        };
        self.clipboard = Some(self.document.outline.copy_subtrees(&[selected]));
        self.refuse(
            &format!(
                "tree copied; {m}-Shift-V pastes a copy, {m}-Alt-Shift-V a clone",
                m = mod_key()
            ),
            cx,
        );
    }

    fn cut_tree(&mut self, _: &CutTree, _: &mut Window, cx: &mut Context<Self>) {
        let (Some(selected), Some(node)) = (self.selected.clone(), self.selected_node()) else {
            return;
        };
        if self.readonly(&node) {
            return self.refuse("@auto subtrees cannot be cut", cx);
        }
        self.clipboard = Some(
            self.document
                .outline
                .copy_subtrees(std::slice::from_ref(&selected)),
        );
        self.document.outline.remove_subtrees(&[selected]);
        self.structure_changed(None, cx);
        self.status = format!(
            "tree cut; {m}-Shift-V pastes copies, {m}-Alt-Shift-V retains identities",
            m = mod_key()
        );
    }

    fn paste_tree(&mut self, as_clone: bool, cx: &mut Context<Self>) {
        let Some(subtree) = self.clipboard.clone() else {
            return self.refuse("tree clipboard is empty", cx);
        };
        if as_clone && subtree.roots.iter().any(|root| self.readonly(&root.node)) {
            return self.refuse(
                "cannot clone @auto derived content; only its root can be cloned",
                cx,
            );
        }
        let (parent, index) = match (&self.selected, self.selected_node()) {
            (Some(selected), Some(node)) => {
                if self.readonly(&node) {
                    return self.refuse("cannot paste beside an @auto subtree", cx);
                }
                let Some((parent, index)) = split_path(selected) else {
                    return;
                };
                (parent, index + 1)
            }
            _ => (None, 0),
        };
        match self
            .document
            .outline
            .paste_subtree(parent.as_ref(), index, &subtree, as_clone)
        {
            Ok(target) => {
                self.structure_changed(Some(target), cx);
                self.status = if as_clone {
                    "pasted clones".into()
                } else {
                    "pasted independent copies".into()
                };
            }
            Err(error) => self.refuse(&error.to_string(), cx),
        }
    }

    fn paste_copy(&mut self, _: &PasteTree, _: &mut Window, cx: &mut Context<Self>) {
        self.paste_tree(false, cx);
    }
    fn paste_clone(&mut self, _: &PasteClone, _: &mut Window, cx: &mut Context<Self>) {
        self.paste_tree(true, cx);
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

    fn insert_sibling(&mut self, _: &InsertSibling, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(current) = self.selected_node()
            && self.readonly(&current)
        {
            return self.refuse("cannot insert beside an @auto descendant", cx);
        }
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
            self.rename_after_select = true;
        }
    }

    fn insert_child(&mut self, _: &InsertChild, _window: &mut Window, cx: &mut Context<Self>) {
        let (Some(selected), Some(parent)) = (self.selected.clone(), self.selected_node()) else {
            return;
        };
        if self.refuses_children(&parent) {
            return self.refuse("this node cannot take new children", cx);
        }
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
            self.rename_after_select = true;
        }
    }

    fn delete_node(&mut self, _: &DeleteNode, _: &mut Window, cx: &mut Context<Self>) {
        let Some(selected) = self.selected.clone() else {
            return;
        };
        if self
            .selected_node()
            .is_some_and(|node| self.readonly(&node))
        {
            return self.refuse("@auto subtrees cannot be deleted", cx);
        }
        let headline = self
            .selected_node()
            .and_then(|id| self.document.outline.nodes.get(&id))
            .map(|node| node.headline.clone())
            .unwrap_or_default();
        if self.apply_op(Operation::Remove { position: selected }, None, cx) {
            self.status = format!("deleted \"{headline}\"");
        }
    }

    /// Jumps to the next (`delta` 1) or previous (-1) search match,
    /// expanding its ancestors so the row is visible.
    fn step_search(&mut self, delta: isize, cx: &mut Context<Self>) {
        // Matches were computed against the outline as it was; recompute so
        // edits between searches can't leave stale positions behind.
        let query = self.search.read(cx).value().to_string();
        self.search_matches = find_positions(&self.document.outline, &query);
        let count = self.search_matches.len();
        if count == 0 {
            return self.refuse("no matches", cx);
        }
        let next = next_match(&self.search_matches, self.selected.as_ref(), delta);
        self.search_cursor = next;
        let target = self.search_matches[next].clone();
        // Expand every ancestor of the target.
        let mut path = String::new();
        let parts: Vec<&str> = target.0.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            if let Some(position) = self.document.outline.position(&PositionId(path.clone())) {
                self.expanded.insert(position.node.clone());
            }
        }
        self.refresh_tree(Some(target.clone()), cx);
        self.tree.update(cx, |state, _| {
            let id: SharedString = target.0.into();
            if let Some(ix) = state.index_of(&id) {
                state.scroll_to_item(ix, ScrollStrategy::Center);
            }
        });
        self.status = format!("match {} of {count}", next + 1);
        cx.notify();
    }

    fn find_focus(&mut self, _: &FindFocus, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.search.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    }

    /// `h` / F2: edit the selected node's headline in place, in its tree row.
    fn rename_inline(&mut self, _: &RenameInline, window: &mut Window, cx: &mut Context<Self>) {
        if self.row_edit.is_some() {
            return;
        }
        let (Some(selected), Some(node_id)) = (self.selected.clone(), self.selected_node()) else {
            return;
        };
        if self.readonly(&node_id) {
            return self.refuse("@auto descendants are read-only", cx);
        }
        let text = self
            .document
            .outline
            .nodes
            .get(&node_id)
            .map(|node| node.headline.clone())
            .unwrap_or_default();
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |state, cx| state.set_value(text, window, cx));
        let subscription = cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } | InputEvent::Blur => {
                    this.finish_row_edit(true, window, cx);
                }
                _ => {}
            },
        );
        let handle = input.read(cx).focus_handle(cx);
        self.row_edit = Some(RowEdit {
            path: selected.0,
            input,
            _subscription: subscription,
        });
        window.focus(&handle, cx);
        if let Some(edit) = &self.row_edit {
            edit.input
                .update(cx, |state, cx| state.select_all(window, cx));
        }
        cx.notify();
    }

    fn cancel_row_edit(&mut self, _: &CancelRowEdit, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_row_edit(false, window, cx);
    }

    /// Ends the inline edit, keeping the new text when `commit`, and hands
    /// focus back to the tree.
    fn finish_row_edit(&mut self, commit: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.row_edit.take() else {
            return;
        };
        if commit && self.selected.as_ref().is_some_and(|id| id.0 == edit.path) {
            let value = edit.input.read(cx).value().to_string();
            if self.edit_selected(cx, |node| &mut node.headline, value.clone()) {
                self.headline
                    .update(cx, |state, cx| state.set_value(value, window, cx));
                let keep = self.selected.clone();
                self.refresh_tree(keep, cx);
            }
        }
        self.tree.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    fn select_row(&mut self, id: &PositionId, cx: &mut Context<Self>) {
        self.tree.update(cx, |state, cx| {
            let id: SharedString = id.0.clone().into();
            if let Some(ix) = state.index_of(&id) {
                state.set_selected_index(Some(ix), cx);
                state.scroll_to_item(ix, ScrollStrategy::Nearest);
            }
        });
    }

    /// Left: collapse an expanded folder, otherwise jump to the parent.
    fn tree_left(&mut self, _: &TreeLeft, _: &mut Window, cx: &mut Context<Self>) {
        let (Some(selected), Some(node)) = (self.selected.clone(), self.selected_node()) else {
            return;
        };
        let has_children = self
            .document
            .outline
            .position(&selected)
            .is_some_and(|p| !p.children.is_empty());
        if has_children && self.expanded.contains(&node) {
            self.toggle_row(&selected.0, cx);
        } else if let Some(parent) = self.document.outline.parent_position(&selected) {
            self.select_row(&parent, cx);
        }
    }

    /// Right: expand a collapsed folder, otherwise jump to its first child.
    fn tree_right(&mut self, _: &TreeRight, _: &mut Window, cx: &mut Context<Self>) {
        let (Some(selected), Some(node)) = (self.selected.clone(), self.selected_node()) else {
            return;
        };
        let has_children = self
            .document
            .outline
            .position(&selected)
            .is_some_and(|p| !p.children.is_empty());
        if !has_children {
            return;
        }
        if self.expanded.contains(&node) {
            self.select_row(&join_path(Some(&selected), 0), cx);
        } else {
            self.toggle_row(&selected.0, cx);
        }
    }

    /// File > Open: pick an outline with the native dialog and load it into
    /// this window. Warns once first if there are unsaved changes.
    fn open_file(&mut self, _: &OpenFile, window: &mut Window, cx: &mut Context<Self>) {
        if !self.confirm_discard(&format!("{}-O", mod_key()), cx) {
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update_in(cx, |this, window, cx| this.open_path(path, window, cx))
                    .ok();
            }
        })
        .detach();
    }

    /// With unsaved changes, warns on the first call and returns false; the
    /// repeat (`again`) discards them and returns true.
    fn confirm_discard(&mut self, again: &str, cx: &mut Context<Self>) -> bool {
        if self.dirty && !self.quit_armed {
            self.quit_armed = true;
            self.status = format!(
                "unsaved changes -- {}-S to save, {again} again to discard",
                mod_key()
            );
            cx.notify();
            return false;
        }
        true
    }

    /// File > Open Recent > N.
    fn open_recent(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.recent.get(index).cloned() else {
            return;
        };
        if self.confirm_discard("the same menu item", cx) {
            self.open_path(path, window, cx);
        }
    }

    /// Replaces the document with the outline at `path`.
    fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        match load_document(&path, self.load_derived) {
            Ok(loaded) => {
                self.document = loaded.document;
                self.path = loaded.path;
                push_recent(&mut self.recent, &self.path);
                save_recent(&self.recent);
                set_menus(cx, &self.recent);
                self.writable_external = loaded.writable_external;
                self.original_external = loaded.original_external;
                self.derived_nodes = loaded.derived_nodes;
                self.status = loaded.status;
                self.dirty = false;
                self.quit_armed = false;
                self.row_edit = None;
                self.clipboard = None;
                self.expanded.clear();
                self.selected = None;
                self.search_matches.clear();
                let first =
                    (!self.document.outline.roots.is_empty()).then(|| PositionId("0".to_owned()));
                if first.is_none() {
                    // Nothing to select, so the selection observer won't clear these.
                    self.headline
                        .update(cx, |state, cx| state.set_value("", window, cx));
                    self.body
                        .update(cx, |state, cx| state.set_value("", window, cx));
                }
                self.refresh_tree(first, cx);
                self.tree.update(cx, |state, cx| state.focus(window, cx));
            }
            Err(error) => self.status = format!("open failed: {error}"),
        }
        cx.notify();
    }

    fn minimize_window(&mut self, _: &MinimizeWindow, window: &mut Window, _: &mut Context<Self>) {
        window.minimize_window();
    }

    fn zoom_window(&mut self, _: &ZoomWindow, window: &mut Window, _: &mut Context<Self>) {
        window.zoom_window();
    }

    fn focus_tree(&mut self, _: &FocusTree, window: &mut Window, cx: &mut Context<Self>) {
        self.tree.update(cx, |state, cx| state.focus(window, cx));
    }

    /// Expands or collapses the row at `path` (chevron click).
    fn toggle_row(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(node) = self
            .document
            .outline
            .position(&PositionId(path.to_owned()))
            .map(|position| position.node.clone())
        else {
            return;
        };
        if !self.expanded.remove(&node) {
            self.expanded.insert(node);
        }
        let keep = self.selected.clone();
        self.refresh_tree(keep, cx);
        cx.notify();
    }

    fn save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        if loaded_is_untitled(&self.path) {
            return self.refuse(
                &format!("no file open -- use File > Open… ({}-O)", mod_key()),
                cx,
            );
        }
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
            self.status = format!(
                "unsaved changes -- {m}-S to save, {m}-Q again to discard",
                m = mod_key()
            );
            self.quit_armed = true;
            cx.notify();
        } else {
            cx.quit();
        }
    }
}

impl Render for GuiApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.open_on_start {
            self.open_on_start = false;
            self.open_file(&OpenFile, window, cx);
        }
        if self.needs_tree_focus {
            self.needs_tree_focus = false;
            self.tree.update(cx, |state, cx| state.focus(window, cx));
        }
        let read_only = self
            .selected_node()
            .is_some_and(|node| self.readonly(&node));
        let theme = cx.theme();
        let (muted, border, sidebar_bg) =
            (theme.muted_foreground, theme.sidebar_border, theme.sidebar);
        let file_name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_owned());

        let title_bar = TitleBar::new().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_sm()
                .child(file_name)
                .when(self.dirty, |title| {
                    title.child(div().size(px(7.)).rounded_full().bg(theme.primary))
                }),
        );

        let hover = theme.muted.opacity(0.35);
        let row_edit = self
            .row_edit
            .as_ref()
            .map(|edit| (edit.path.clone(), edit.input.clone()));
        let clone_paths = self.clone_paths.clone();
        let app = cx.entity().downgrade();
        let tree_state = self.tree.clone();
        let accent = theme.primary;
        let sidebar = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(sidebar_bg)
            .child(
                div().key_context("SearchBox").p_2().child(
                    gpui_kit::component::input::Input::new(&self.search)
                        .prefix(Icon::new(IconName::Search).size_4().text_color(muted))
                        .cleanable(true),
                ),
            )
            .child(div().flex_1().min_h_0().px_1().child(tree(
                &self.tree,
                move |ix, entry, selected, _, _| {
                    let item = entry.item();
                    let path = item.id.to_string();
                    let chevron = if entry.is_folder() {
                        let name = if entry.is_expanded() {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        };
                        // Only the chevron toggles; a click elsewhere on the row
                        // just selects it (the tree widget would do both).
                        let app = app.clone();
                        let path = path.clone();
                        div()
                            .id(("chevron", ix))
                            .p_0p5()
                            .on_click(move |_, _, cx| {
                                cx.stop_propagation();
                                app.update(cx, |this, cx| this.toggle_row(&path, cx)).ok();
                            })
                            .child(Icon::new(name).size_3().text_color(muted))
                            .into_any_element()
                    } else {
                        div().w(px(12.)).into_any_element()
                    };
                    let kind = if entry.is_folder() {
                        if entry.is_expanded() {
                            IconName::FolderOpen
                        } else {
                            IconName::FolderClosed
                        }
                    } else {
                        IconName::File
                    };
                    let is_clone = clone_paths.contains(item.id.as_ref());
                    let editing = row_edit
                        .as_ref()
                        .filter(|(edit_path, _)| *edit_path == path);
                    let label = match editing {
                        Some((_, input)) => div()
                            .key_context("TreeRowEdit")
                            .flex_1()
                            .min_w_0()
                            .child(gpui_kit::component::input::Input::new(input).small())
                            .into_any_element(),
                        None => div().child(item.label.clone()).into_any_element(),
                    };
                    let is_editing = editing.is_some();
                    let tree_state = tree_state.clone();
                    let row_app = app.clone();
                    let row_path = path.clone();
                    let folder = entry.is_folder();
                    ListItem::new(ix)
                        .selected(selected)
                        .on_click(move |event, window, cx| {
                            cx.stop_propagation();
                            if is_editing {
                                return;
                            }
                            tree_state.update(cx, |state, cx| {
                                state.set_selected_index(Some(ix), cx);
                                state.focus(window, cx);
                            });
                            // Double-click a folder to expand or collapse it.
                            if folder && event.click_count() >= 2 {
                                row_app
                                    .update(cx, |this, cx| this.toggle_row(&row_path, cx))
                                    .ok();
                            }
                        })
                        .rounded_md()
                        .py_0p5()
                        .pl(px(6. + 14. * entry.depth() as f32))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .text_sm()
                                .child(chevron)
                                .child(Icon::new(kind).size_4().text_color(muted))
                                .child(label)
                                .when(is_clone, |row| {
                                    row.child(Icon::new(IconName::Copy).size_3().text_color(accent))
                                }),
                        )
                },
            )));

        let main = div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_6()
                    .pt_4()
                    .pb_2()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .rounded_md()
                            .hover(|style| style.bg(hover))
                            .child(
                                gpui_kit::component::input::Input::new(&self.headline)
                                    .large()
                                    .appearance(false)
                                    .bordered(false)
                                    .text_2xl()
                                    .font_semibold()
                                    .readonly(read_only),
                            ),
                    )
                    .when(read_only, |header| {
                        header.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .px_2()
                                .py_0p5()
                                .rounded_full()
                                .border_1()
                                .border_color(border)
                                .text_xs()
                                .text_color(muted)
                                .child(Icon::new(IconName::Eye).size_3().text_color(muted))
                                .child("Read-only · derived"),
                        )
                    }),
            )
            .child(div().mx_6().h(px(1.)).bg(border))
            .child(
                div().flex_1().min_h_0().px_4().py_2().child(
                    Editor::new(&self.body)
                        .h(relative(1.))
                        .appearance(false)
                        .bordered(false)
                        .readonly(read_only),
                ),
            );

        let split = h_resizable("main-split")
            .child(
                resizable_panel()
                    .size(px(300.))
                    .size_range(px(200.)..px(520.))
                    .child(sidebar),
            )
            .child(resizable_panel().child(main));

        let status_bar = div()
            .h(px(26.))
            .px_3()
            .flex()
            .items_center()
            .bg(sidebar_bg)
            .border_t_1()
            .border_color(border)
            .text_xs()
            .text_color(muted)
            .child(self.status.clone());

        div()
            .key_context("LeoGui")
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::quit))
            .on_action(cx.listener(Self::insert_sibling))
            .on_action(cx.listener(Self::insert_child))
            .on_action(cx.listener(Self::delete_node))
            .on_action(cx.listener(Self::find_focus))
            .on_action(cx.listener(Self::rename_inline))
            .on_action(cx.listener(Self::cancel_row_edit))
            .on_action(cx.listener(Self::tree_left))
            .on_action(cx.listener(Self::tree_right))
            .on_action(cx.listener(Self::focus_tree))
            .on_action(cx.listener(Self::minimize_window))
            .on_action(cx.listener(Self::open_file))
            .on_action(cx.listener(|this, _: &OpenRecent0, w, cx| this.open_recent(0, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent1, w, cx| this.open_recent(1, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent2, w, cx| this.open_recent(2, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent3, w, cx| this.open_recent(3, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent4, w, cx| this.open_recent(4, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent5, w, cx| this.open_recent(5, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent6, w, cx| this.open_recent(6, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent7, w, cx| this.open_recent(7, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent8, w, cx| this.open_recent(8, w, cx)))
            .on_action(cx.listener(|this, _: &OpenRecent9, w, cx| this.open_recent(9, w, cx)))
            .on_action(cx.listener(Self::zoom_window))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::copy_tree))
            .on_action(cx.listener(Self::cut_tree))
            .on_action(cx.listener(Self::paste_copy))
            .on_action(cx.listener(Self::paste_clone))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(title_bar)
            .child(div().flex_1().min_h_0().child(split))
            .child(status_bar)
    }
}

/// Paths of every row whose node is referenced by more than one position.
fn find_clone_paths(outline: &Outline) -> HashSet<String> {
    fn count(positions: &[Position], counts: &mut HashMap<NodeId, usize>) {
        for position in positions {
            *counts.entry(position.node.clone()).or_default() += 1;
            count(&position.children, counts);
        }
    }
    fn collect(
        position: &Position,
        path: String,
        counts: &HashMap<NodeId, usize>,
        found: &mut HashSet<String>,
    ) {
        if counts.get(&position.node).is_some_and(|n| *n > 1) {
            found.insert(path.clone());
        }
        for (index, child) in position.children.iter().enumerate() {
            collect(child, format!("{path}/{index}"), counts, found);
        }
    }
    let mut counts = HashMap::new();
    count(&outline.roots, &mut counts);
    let mut found = HashSet::new();
    for (index, root) in outline.roots.iter().enumerate() {
        collect(root, index.to_string(), &counts, &mut found);
    }
    found
}

const MAX_RECENT: usize = 10;

/// The shortcut modifier as the user sees it: Cmd on macOS, Ctrl elsewhere
/// (the bindings use `secondary-`, which GPUI maps the same way).
fn mod_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

/// True for the placeholder path of a window started without a file.
fn loaded_is_untitled(path: &Path) -> bool {
    path.as_os_str().is_empty()
}

/// Where the recent-files list lives: `~/.config/leo-cub/recent.txt`, or
/// `%APPDATA%\\leo-cub\\recent.txt` on Windows.
fn recent_file() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)?
    } else {
        PathBuf::from(std::env::var_os("HOME")?).join(".config")
    };
    Some(base.join("leo-cub").join("recent.txt"))
}

/// Parses the recent-files list: one path per line, blanks and missing
/// files dropped, newest first, at most [`MAX_RECENT`].
fn parse_recent(text: &str, exists: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| exists(path) && seen.insert(path.clone()))
        .take(MAX_RECENT)
        .collect()
}

fn load_recent() -> Vec<PathBuf> {
    recent_file()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(|text| parse_recent(&text, Path::exists))
        .unwrap_or_default()
}

/// Moves `path` to the front of `recent` (absolute, de-duplicated, capped).
fn push_recent(recent: &mut Vec<PathBuf>, path: &Path) {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    recent.retain(|existing| *existing != path);
    recent.insert(0, path);
    recent.truncate(MAX_RECENT);
}

fn save_recent(recent: &[PathBuf]) {
    let Some(file) = recent_file() else { return };
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text: String = recent
        .iter()
        .map(|path| format!("{}\n", path.display()))
        .collect();
    let _ = std::fs::write(file, text);
}

/// One "Open Recent" item per remembered file, labelled with its name.
fn recent_menu_items(recent: &[PathBuf]) -> Vec<MenuItem> {
    fn item(index: usize, label: String) -> MenuItem {
        match index {
            0 => MenuItem::action(label, OpenRecent0),
            1 => MenuItem::action(label, OpenRecent1),
            2 => MenuItem::action(label, OpenRecent2),
            3 => MenuItem::action(label, OpenRecent3),
            4 => MenuItem::action(label, OpenRecent4),
            5 => MenuItem::action(label, OpenRecent5),
            6 => MenuItem::action(label, OpenRecent6),
            7 => MenuItem::action(label, OpenRecent7),
            8 => MenuItem::action(label, OpenRecent8),
            _ => MenuItem::action(label, OpenRecent9),
        }
    }
    recent
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let name = path.file_name().map_or_else(
                || path.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            item(index, name)
        })
        .collect()
}

/// The macOS menu bar. Every item is an action the window already handles,
/// so the menu and the keyboard shortcuts stay in step.
fn set_menus(cx: &mut App, recent: &[PathBuf]) {
    cx.set_menus(vec![
        Menu {
            name: "Leo Cub".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Leo Cub", Quit),
            ],
            disabled: false,
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open…", OpenFile),
                MenuItem::submenu(Menu {
                    name: "Open Recent".into(),
                    items: recent_menu_items(recent),
                    disabled: recent.is_empty(),
                }),
                MenuItem::separator(),
                MenuItem::action("Save", Save),
            ],
            disabled: false,
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("New Sibling", InsertSibling),
                MenuItem::action("New Child", InsertChild),
                MenuItem::action("Rename", RenameInline),
                MenuItem::action("Delete", DeleteNode),
                MenuItem::separator(),
                MenuItem::action("Copy Subtree", CopyTree),
                MenuItem::action("Cut Subtree", CutTree),
                MenuItem::action("Paste Subtree", PasteTree),
                MenuItem::action("Paste as Clone", PasteClone),
            ],
            disabled: false,
        },
        Menu {
            name: "Outline".into(),
            items: vec![
                MenuItem::action("Move Up", MoveUp),
                MenuItem::action("Move Down", MoveDown),
                MenuItem::action("Promote (Move Left)", MoveLeft),
                MenuItem::action("Demote (Move Right)", MoveRight),
                MenuItem::separator(),
                MenuItem::action("Find", FindFocus),
            ],
            disabled: false,
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", MinimizeWindow),
                MenuItem::action("Zoom", ZoomWindow),
            ],
            disabled: false,
        },
    ]);
}

/// Registers cub's own tree-sitter grammars with the editor. cub and the
/// editor share one tree-sitter runtime, so a grammar crate's `LANGUAGE`
/// converts straight into the editor's `Language`.
fn register_languages() {
    let registry = LanguageRegistry::singleton();
    let add = |name: &str, language: tree_sitter::Language, highlights: &str| {
        registry.register(
            name,
            &LanguageConfig::new(name, language, vec![], highlights, "", ""),
        );
    };
    add(
        "rust",
        tree_sitter_rust::LANGUAGE.into(),
        tree_sitter_rust::HIGHLIGHTS_QUERY,
    );
    add(
        "python",
        tree_sitter_python::LANGUAGE.into(),
        tree_sitter_python::HIGHLIGHTS_QUERY,
    );
    add(
        "javascript",
        tree_sitter_javascript::LANGUAGE.into(),
        tree_sitter_javascript::HIGHLIGHT_QUERY,
    );
    add(
        "go",
        tree_sitter_go::LANGUAGE.into(),
        tree_sitter_go::HIGHLIGHTS_QUERY,
    );
    add(
        "typescript",
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
    );
    add(
        "tsx",
        tree_sitter_typescript::LANGUAGE_TSX.into(),
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
    );
    add(
        "markdown",
        tree_sitter_md::LANGUAGE.into(),
        tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
    );
}

/// Maps a `@language` name or file extension to a registered grammar name.
fn canonical_language(name: &str) -> Option<&'static str> {
    Some(
        match name.trim().trim_start_matches('.').to_lowercase().as_str() {
            "rust" | "rs" => "rust",
            "python" | "py" => "python",
            "javascript" | "js" | "jsx" | "mjs" => "javascript",
            "typescript" | "ts" => "typescript",
            "tsx" => "tsx",
            "go" => "go",
            "markdown" | "md" | "rmd" => "markdown",
            _ => return None,
        },
    )
}

/// The `@language NAME` directive in `body`, if any.
fn language_directive(body: &str) -> Option<&str> {
    body.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("@language")?;
        rest.split_whitespace().next()
    })
}

/// Which grammar to highlight the node at `position` with: the nearest
/// ancestor-or-self `@language` directive, else the extension of the
/// nearest `@file`-style headline's filename; `"text"` when neither maps to
/// a grammar cub ships. Same precedence as the TUI.
fn language_for(outline: &Outline, position: &PositionId) -> &'static str {
    let mut found: Option<&'static str> = None;
    let mut prefix = String::new();
    for component in position.0.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        let Some(node) = outline
            .position(&PositionId(prefix.clone()))
            .and_then(|p| outline.nodes.get(&p.node))
        else {
            break;
        };
        if let Some(filename) = external_filename(&node.headline)
            && let Some(ext) = filename.rsplit_once('.').map(|(_, ext)| ext)
        {
            found = canonical_language(ext).or(found);
        }
        if let Some(name) = language_directive(&node.body) {
            found = canonical_language(name).or(found);
        }
    }
    found.unwrap_or("text")
}

/// Everything needed to show one outline.
struct Loaded {
    document: LeoDocument,
    path: PathBuf,
    writable_external: HashMap<NodeId, WritableExternalFile>,
    original_external: OriginalExternalState,
    derived_nodes: HashSet<NodeId>,
    status: String,
}

/// Opens `path` and, unless `load_derived` is off, loads its derived files.
fn load_document(path: &Path, load_derived: bool) -> Result<Loaded> {
    let mut document = LeoDocument::open(path)?;
    let (status, writable_external, original_external, derived_nodes) = if load_derived {
        let report = load_derived_files(&mut document.outline, path);
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
            report.derived_nodes,
        )
    } else {
        (
            "derived files disabled".to_owned(),
            HashMap::new(),
            OriginalExternalState::default(),
            HashSet::new(),
        )
    };
    Ok(Loaded {
        document,
        path: path.to_path_buf(),
        writable_external,
        original_external,
        derived_nodes,
        status,
    })
}

/// The placeholder state of a window started without a file.
fn untitled() -> Loaded {
    Loaded {
        document: LeoDocument::empty(),
        path: PathBuf::new(),
        writable_external: HashMap::new(),
        original_external: OriginalExternalState::default(),
        derived_nodes: HashSet::new(),
        status: format!("no file open -- File > Open… ({}-O)", mod_key()),
    }
}

pub fn run(path: Option<PathBuf>, load_derived: bool) -> Result<()> {
    let loaded = match &path {
        Some(path) => load_document(path, load_derived)?,
        None => untitled(),
    };
    let mut recent = load_recent();
    if let Some(path) = &path {
        push_recent(&mut recent, path);
        save_recent(&recent);
    }
    let mut seed = Some((loaded, load_derived, recent.clone()));
    let menu_recent = recent.clone();
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            register_languages();
            // A single-window app: closing the window quits.
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.bind_keys([
                KeyBinding::new("secondary-s", Save, None),
                KeyBinding::new("secondary-o", OpenFile, None),
                KeyBinding::new("secondary-m", MinimizeWindow, None),
                KeyBinding::new("secondary-q", Quit, None),
                KeyBinding::new("secondary-i", InsertSibling, None),
                KeyBinding::new("secondary-shift-i", InsertChild, None),
                KeyBinding::new("secondary-backspace", DeleteNode, None),
                KeyBinding::new("secondary-f", FindFocus, None),
                KeyBinding::new("f2", RenameInline, None),
                KeyBinding::new("h", RenameInline, Some("Tree")),
                // The inline edit input lives inside the tree, so the bare
                // `h` shortcut would otherwise swallow the letter there.
                KeyBinding::new("h", NoAction, Some("TreeRowEdit > Input")),
                KeyBinding::new("left", TreeLeft, Some("Tree")),
                KeyBinding::new("right", TreeRight, Some("Tree")),
                KeyBinding::new("escape", FocusTree, Some("SearchBox > Input")),
                KeyBinding::new("escape", CancelRowEdit, Some("TreeRowEdit > Input")),
                KeyBinding::new("secondary-alt-up", MoveUp, None),
                KeyBinding::new("secondary-alt-down", MoveDown, None),
                KeyBinding::new("secondary-alt-left", MoveLeft, None),
                KeyBinding::new("secondary-alt-right", MoveRight, None),
                KeyBinding::new("secondary-shift-c", CopyTree, None),
                KeyBinding::new("secondary-shift-x", CutTree, None),
                KeyBinding::new("secondary-shift-v", PasteTree, None),
                KeyBinding::new("secondary-alt-shift-v", PasteClone, None),
            ]);
            let (loaded, load_derived, recent) = seed.take().expect("run once");
            gpui_kit::open_window(TitleBar::window_options(), cx, move |window, cx| {
                Theme::sync_system_appearance(Some(window), cx);
                cx.new(|cx| GuiApp::new(loaded, load_derived, recent, window, cx))
            })
            .expect("failed to open window");
            // After the key bindings, so the menus can show their shortcuts.
            set_menus(cx, &menu_recent);
            cx.activate(true);
        });
    Ok(())
}

/// Every position whose headline or body contains `query`
/// (case-insensitive), in outline order.
fn find_positions(outline: &Outline, query: &str) -> Vec<PositionId> {
    let query = query.to_lowercase();
    let mut found = Vec::new();
    if query.is_empty() {
        return found;
    }
    fn visit(
        outline: &Outline,
        position: &Position,
        path: String,
        query: &str,
        found: &mut Vec<PositionId>,
    ) {
        if let Some(node) = outline.nodes.get(&position.node)
            && (node.headline.to_lowercase().contains(query)
                || node.body.to_lowercase().contains(query))
        {
            found.push(PositionId(path.clone()));
        }
        for (index, child) in position.children.iter().enumerate() {
            visit(outline, child, format!("{path}/{index}"), query, found);
        }
    }
    for (index, root) in outline.roots.iter().enumerate() {
        visit(outline, root, index.to_string(), &query, &mut found);
    }
    found
}

fn path_key(id: &PositionId) -> Vec<usize> {
    id.0.split('/').filter_map(|p| p.parse().ok()).collect()
}

/// Index into `matches` (in outline order) to jump to from `selected`:
/// the neighbouring match in direction `delta` (wrapping), or -- when the
/// selection isn't on a match -- the nearest match after (forward) or
/// before (backward) it.
fn next_match(matches: &[PositionId], selected: Option<&PositionId>, delta: isize) -> usize {
    let count = matches.len();
    let Some(selected) = selected else {
        return if delta >= 0 { 0 } else { count - 1 };
    };
    let key = path_key(selected);
    let keys: Vec<Vec<usize>> = matches.iter().map(path_key).collect();
    if let Some(ix) = keys.iter().position(|k| *k == key) {
        return (ix as isize + delta).rem_euclid(count as isize) as usize;
    }
    if delta >= 0 {
        keys.iter().position(|k| *k > key).unwrap_or(0)
    } else {
        keys.iter().rposition(|k| *k < key).unwrap_or(count - 1)
    }
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
    fn language_follows_directive_then_file_extension_down_the_tree() {
        let mut o = outline();
        // a (child b), c
        o.nodes.get_mut(&NodeId("a".into())).unwrap().headline = "@file src/lib.rs".into();
        assert_eq!(language_for(&o, &PositionId("0/0".into())), "rust");
        assert_eq!(language_for(&o, &PositionId("1".into())), "text");
        // a closer @language directive wins over the inherited extension
        o.nodes.get_mut(&NodeId("b".into())).unwrap().body = "@language python\nx = 1".into();
        assert_eq!(language_for(&o, &PositionId("0/0".into())), "python");
        // an unknown language leaves the inherited one in place
        o.nodes.get_mut(&NodeId("b".into())).unwrap().body = "@language cobol".into();
        assert_eq!(language_for(&o, &PositionId("0/0".into())), "rust");
        assert_eq!(canonical_language(".TS"), Some("typescript"));
        assert_eq!(language_directive("  @language  go  \nx"), Some("go"));
    }

    #[::core::prelude::v1::test]
    fn clones_are_marked_at_every_occurrence() {
        let mut o = outline();
        // c appears again under a
        o.roots[0].children.push(leaf("c"));
        let found = find_clone_paths(&o);
        let mut found: Vec<_> = found.into_iter().collect();
        found.sort();
        assert_eq!(found, ["0/1", "1"]);
        assert!(find_clone_paths(&outline()).is_empty());
    }

    #[::core::prelude::v1::test]
    fn recent_list_parses_dedupes_and_caps() {
        let text = "/a.leo\n\n  /b.leo  \n/a.leo\n/gone.leo\n";
        let kept = parse_recent(text, |p| p != Path::new("/gone.leo"));
        assert_eq!(kept, [PathBuf::from("/a.leo"), PathBuf::from("/b.leo")]);

        let many: String = (0..30).map(|i| format!("/f{i}.leo\n")).collect();
        assert_eq!(parse_recent(&many, |_| true).len(), MAX_RECENT);
    }

    #[::core::prelude::v1::test]
    fn pushing_a_recent_file_moves_it_to_the_front() {
        let mut recent = vec![PathBuf::from("/x.leo"), PathBuf::from("/y.leo")];
        push_recent(&mut recent, Path::new("/y.leo"));
        assert_eq!(recent, [PathBuf::from("/y.leo"), PathBuf::from("/x.leo")]);
        for i in 0..20 {
            push_recent(&mut recent, Path::new(&format!("/n{i}.leo")));
        }
        assert_eq!(recent.len(), MAX_RECENT);
        assert_eq!(recent[0], PathBuf::from("/n19.leo"));
    }

    #[::core::prelude::v1::test]
    fn an_empty_path_means_untitled() {
        assert!(loaded_is_untitled(Path::new("")));
        assert!(!loaded_is_untitled(Path::new("a.leo")));
    }

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
    fn find_matches_headlines_and_bodies_case_insensitively_in_order() {
        let mut o = outline();
        o.nodes.get_mut(&NodeId("c".into())).unwrap().body = "Needle here".into();
        o.nodes.get_mut(&NodeId("b".into())).unwrap().headline = "has NEEDLE".into();
        assert_eq!(
            find_positions(&o, "needle"),
            [PositionId("0/0".into()), PositionId("1".into())]
        );
        assert!(find_positions(&o, "").is_empty());
        assert!(find_positions(&o, "absent").is_empty());
    }

    #[::core::prelude::v1::test]
    fn next_match_walks_outline_order_not_string_order() {
        let m = |s: &str| PositionId(s.into());
        let matches = [m("1"), m("2"), m("10")];
        assert_eq!(next_match(&matches, None, 1), 0);
        assert_eq!(next_match(&matches, None, -1), 2);
        assert_eq!(next_match(&matches, Some(&m("2")), 1), 2);
        assert_eq!(next_match(&matches, Some(&m("10")), 1), 0);
        assert_eq!(next_match(&matches, Some(&m("1")), -1), 2);
        // off a match: nearest in the requested direction, numerically
        assert_eq!(next_match(&matches, Some(&m("3")), 1), 2);
        assert_eq!(next_match(&matches, Some(&m("3")), -1), 1);
        assert_eq!(next_match(&matches, Some(&m("1/4")), 1), 1);
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
    fn empty_headlines_get_a_visible_label() {
        let mut o = outline();
        o.nodes.get_mut(&NodeId("c".into())).unwrap().headline = "  ".into();
        let items = build_items(&o, &HashSet::new());
        assert_eq!(items[1].label.as_ref(), "(empty)");
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
