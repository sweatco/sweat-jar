pub mod api;
pub mod model_legacy;
mod model_v1;
pub mod tests;
mod versioned_model;
pub mod view;

pub mod model {
    pub use super::{model_v1::*, versioned_model::*};
}
