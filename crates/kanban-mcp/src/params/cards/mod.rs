use serde::{Deserialize, Deserializer};

mod common;
mod create;
mod graph;
mod list;
mod update;

pub(crate) use common::{BoardParam, KeyParams};
pub(crate) use create::{CreateBacklogCardParams, CreateCardItem, CreateCardsParams, CreateParams};
pub(crate) use graph::DependencyGraphParams;
pub(crate) use list::ListParams;
pub(crate) use update::UpdateParams;

fn nullable_i64_patch<'de, D>(deserializer: D) -> Result<Option<Option<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<i64>::deserialize(deserializer).map(Some)
}
