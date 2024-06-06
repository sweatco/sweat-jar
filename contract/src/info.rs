use sweat_jar_model::api::InfoApi;

use crate::{Contract, PACKAGE_NAME, VERSION};

impl InfoApi for Contract {
    fn contract_version(&self) -> String {
        format!("{PACKAGE_NAME}-{VERSION}")
    }

    fn contract_build_date(&self) -> String {
        compile_time::datetime_str!().to_string()
    }
}

#[cfg(test)]
mod test {
    use sweat_jar_model::api::InfoApi;

    use crate::{common::tests::Context, test_utils::admin};

    #[test]
    fn test_contract_version_and_date() {
        let admin = admin();
        let context = Context::new(admin);
        assert_eq!(context.contract().contract_version(), "sweat_jar-2.1.0");
        assert_eq!(context.contract().contract_build_date(), compile_time::datetime_str!());
    }
}
