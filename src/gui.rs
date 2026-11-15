//! Native GPUI desktop frontend (`cub gui FILE`).
//!
//! Sits next to the TUI and shares the `leo` library for everything that
//! touches the document: loading, derived/external file sync, and saving.
//! MVP scope: outline tree, headline + body editing, save.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::Result;
use gpui_kit::component::input::{InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::{TreeItem, TreeState, tree};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;
use leo::{
    LeoDocument, NodeId, OriginalExternalState, Outline, Position, PositionId,
    WritableExternalFile, load_derived_files, save_document,
};

actions!(leo_gui, [Save, Quit]);

struct GuiApp {
    document: LeoDocument,
    path: PathBuf,
    writable_external: HashMap<NodeId, WritableExternalFile>,
    original_external: OriginalExternalState,
    dirty: bool,
    status: String,
    /// Positions the user has expanded; survives tree rebuilds.
    expanded: HashSet<String>,
    selected: Option<PositionId>,
    tree: Entity<TreeState>,
    headline: Entity<InputState>,
    body: Entity<TextareaState>,
    _subscriptions: Vec<Subscription>,
}

fn build_items(outline: &Outline, expanded: &HashSet<String>) -> Vec<TreeItem> {
    fn build(
        outline: &Outline,
        position: &Position,
        path: String,
        expanded: &HashSet<String>,
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
            .expanded(expanded.contains(&path))
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
        subscriptions.push(cx.subscribe(&tree, |this, _, event, _| match event {
            gpui_kit::component::tree::TreeEvent::Expanded(id) => {
                this.expanded.insert(id.to_string());
            }
            gpui_kit::component::tree::TreeEvent::Collapsed(id) => {
                this.expanded.remove(id.as_ref());
            }
        }));
        subscriptions.push(cx.subscribe(&headline, |this, state, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = state.read(cx).value().to_string();
                if this.edit_selected(cx, |node| &mut node.headline, value) {
                    this.refresh_tree(cx);
                }
            }
        }));
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

    /// Rebuilds the tree rows (e.g. after a headline edit) keeping the
    /// expansion state and the current selection. `self.selected` is left
    /// untouched so the selection observer sees no change and does not
    /// reset the editors mid-typing.
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let items = build_items(&self.document.outline, &self.expanded);
        let selected = self.selected.as_ref().map(|id| id.0.clone());
        self.tree.update(cx, |state, cx| {
            state.set_items(items, cx);
            if let Some(id) = selected {
                let id: SharedString = id.into();
                if let Some(ix) = state.index_of(&id) {
                    state.set_selected_index(Some(ix), cx);
                }
            }
        });
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
                self.status = format!("saved {}", self.path.display());
            }
            Err(error) => self.status = format!("save failed: {error}"),
        }
        cx.notify();
    }

    fn quit(&mut self, _: &Quit, _: &mut Window, cx: &mut Context<Self>) {
        if self.dirty {
            self.status = "unsaved changes -- Cmd-S to save, Cmd-Q again to discard".into();
            self.dirty = false; // second Cmd-Q quits
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
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .child(Textarea::new(&self.body)),
                            ),
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
