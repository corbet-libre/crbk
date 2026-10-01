//! Canonical group defaults preserve the decisions and explicit open questions.

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn decided_defaults_preserve_disabled_seats_and_open_questions() {
    let values = crbk::group_defaults().unwrap();
    assert_eq!(values["groups.cap"], 100);
    assert_eq!(values["groups.open_days"], 21);
    assert_eq!(values["groups.shrink_public"], 36);
    assert_eq!(values["groups.shrink_ingroup"], 10);
    assert_eq!(values["seats.enabled"], false);
    assert_eq!(values["groups.reciprocity_credit.enabled"], true);
    assert_eq!(values["waves.group_credit_week_cap"], 1.5);
    assert_eq!(values["waves.welcome_credit"], 0.25);
    assert_eq!(values["waves.introduction_credit"], 1.0);
    for unresolved in [
        "groups.curve",
        "groups.fork_cost_sp",
        "seats.fork_cost",
        "groups.split_public_min",
    ] {
        assert!(!values.contains_key(unresolved));
    }
}
