/// The right noun form for a count: singular when `n == 1`, plural otherwise (so `0`
/// reads "0 families"). Returns just the noun — the caller prints the number.
pub(crate) fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 {
        one
    } else {
        many
    }
}
