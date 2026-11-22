//! Import from other tools: a source builds an [`ImportPlan`] (a dry run that writes nothing), and
//! [`apply`] writes exactly that plan in one transaction.

#[path = "import_csv.rs"]
pub mod csv;
#[path = "import_linear.rs"]
pub mod linear;
#[path = "import_plan.rs"]
mod plan;

pub use plan::{
    Applied, ImportPlan, PlanCycle, PlanIssue, PlanRelation, PlanStatus, Skipped, apply,
};
