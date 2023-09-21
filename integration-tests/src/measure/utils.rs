pub(crate) fn generate_permutations<One, Two>(one: &[One], two: &[Two]) -> Vec<(One, Two)>
where
    One: Copy,
    Two: Copy,
{
    one.into_iter()
        .flat_map(|&item1| two.iter().map(move |&item2| (item1, item2)))
        .collect()
}

pub(crate) fn generate_permutations_3<One, Two, Three>(
    one: &[One],
    two: &[Two],
    three: &[Three],
) -> Vec<(One, Two, Three)>
where
    One: Copy,
    Two: Copy,
    Three: Copy,
{
    one.into_iter()
        .flat_map(|&item1| {
            two.iter()
                .flat_map(move |&item2| three.iter().map(move |&item3| (item1, item2, item3)))
        })
        .collect()
}

#[cfg(test)]
mod test {
    use crate::measure::utils::{generate_permutations, generate_permutations_3};

    #[test]
    fn permutations() -> anyhow::Result<()> {
        assert_eq!(
            generate_permutations(&['a', 'b'], &[10, 20]),
            vec![('a', 10,), ('a', 20,), ('b', 10,), ('b', 20,),]
        );

        assert_eq!(
            generate_permutations_3(&['a', 'b'], &[10, 20], &[true, false]),
            vec![
                ('a', 10, true,),
                ('a', 10, false,),
                ('a', 20, true,),
                ('a', 20, false,),
                ('b', 10, true,),
                ('b', 10, false,),
                ('b', 20, true,),
                ('b', 20, false,),
            ]
        );

        Ok(())
    }
}
