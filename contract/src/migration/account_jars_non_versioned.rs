use near_sdk::near;
use sweat_jar_model::jar::JarId;

use crate::jar::model::Jar;

#[near]
#[derive(Default, Clone)]
pub struct AccountJarsNonVersioned {
    pub last_id: JarId,
    pub jars: Vec<Jar>,
}
