#[cfg(test)]
mod test {

    use crate::{common::tests::Context, test_utils::admin};

    #[test]
    fn test_contract_version() {
        let admin = admin();
        let context = Context::new(admin);
        assert_eq!(context.contract().contract_version(), "sweat_jar-2.1.0");
    }
}
