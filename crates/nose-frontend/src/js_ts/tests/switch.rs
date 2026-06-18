use super::support::switch_labels_for_return;

#[test]
fn stacked_switch_cases_share_the_following_body() {
    let src = "function f(x) { switch (x) { case 1: case 2: return 7; default: return 0; } }";
    assert_eq!(switch_labels_for_return(src, 7), vec![1, 2]);
}
