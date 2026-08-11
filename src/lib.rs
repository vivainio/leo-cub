//! Clone-aware, automation-safe manipulation of Leo XML outlines.

mod model;
mod operation;
mod xml;

pub use model::{Node, NodeId, Outline, Position, PositionId, ValidationError};
pub use operation::{ApplyError, ApplyReport, Operation, OperationBatch, Target};
pub use xml::{LeoDocument, LeoXmlError};
