//! Clone-aware, automation-safe manipulation of Leo XML outlines.

mod auto;
mod clean;
mod derived;
mod import;
mod inspect;
mod model;
mod operation;
mod relative;
mod sync;
mod tree;
mod xml;

pub use auto::{AutoError, AutoFile};
pub use clean::propagate_clean_changes;
pub use derived::{DerivedFile, SentinelError};
pub use import::{ImportError, ImportItem, ImportMode, ImportOptions, ImportReport, import_files};
pub use inspect::{
    ExternalFilter, InspectError, InspectSelector, JsonTree, JsonTreeNode, SearchExcerpt,
    SearchMatch, json_tree, load_matching_external_files, render_compact, render_outline,
    render_outline_with_options, render_search_compact, search_outline, select_subtrees,
};
pub use model::{Node, NodeId, Outline, Position, PositionId, ValidationError, referenced_nodes};
pub use operation::{ApplyError, ApplyReport, Operation, OperationBatch, Target, TreeNode};
pub use relative::RelativeFile;
pub use sync::{
    ExternalFormat, ExternalUpdate, OriginalExternalState, SyncError, SyncItem, SyncReport,
    WritableExternalFile, comment_delimiters, external_file_path, external_snapshot,
    format_for_directive, prepare_external_updates, render_relative, render_thin,
    restore_external_state, sync_document, write_external_updates,
};
pub use tree::{AddPathsReport, HeadlinePathError};
pub use xml::{LeoDocument, LeoXmlError};
