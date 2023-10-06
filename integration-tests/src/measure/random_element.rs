use fake::Fake;

pub trait RandomElement<T> {
    fn random_element(&self) -> &T;
}

impl<T> RandomElement<T> for Vec<T> {
    fn random_element(&self) -> &T {
        &self[(0..self.len()).fake::<usize>()]
    }
}

#[test]
fn test_random_element() {
    use fake::faker::lorem::en::*;
    let vec: Vec<String> = Words(5..10).fake();

    for _ in 0..50 {
        assert!(vec.contains(vec.random_element()));
    }

    let vec = vec!["a"];

    for _ in 0..5 {
        assert!(vec.contains(vec.random_element()));
    }
}
