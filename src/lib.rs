//! Clone-aware, automation-safe manipulation of Leo XML outlines.

mod clean;
mod derived;
mod model;
mod operation;
mod sync;
mod xml;

pub use clean::propagate_clean_changes;
pub use derived::{DerivedFile, SentinelError};
pub use model::{Node, NodeId, Outline, Position, PositionId, ValidationError};
pub use operation::{ApplyError, ApplyReport, Operation, OperationBatch, Target};
pub use sync::{SyncError, SyncItem, SyncReport, sync_document};
pub use xml::{LeoDocument, LeoXmlError};
