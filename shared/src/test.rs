use super::*;

#[test]
fn min_contribution_is_usable() {
    let amount = MIN_CONTRIBUTION;
    assert_eq!(amount.checked_add_amount(1), Some(2));
}

#[test]
fn checked_add_amount_reports_overflow() {
    assert_eq!(2i128.checked_add_amount(3), Some(5));
    assert_eq!(i128::MAX.checked_add_amount(1), None);
}
