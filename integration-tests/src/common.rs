use sweat_jar_model::{data::jar::JarView, TokenAmount};

pub(crate) fn total_principal(jars: &Vec<JarView>) -> TokenAmount {
    jars.iter().map(|jar| jar.principal.0).sum()
}
