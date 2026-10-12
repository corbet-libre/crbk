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
    // Keys the group catalogue first lacked now carry the GROUPS-DESIGN defaults.
    assert_eq!(values["groups.split_public_min"], 43);
    assert_eq!(values["groups.fork_cost_sp"], 2);
    assert_eq!(values["groups.suggest.threshold"], 66);
    assert_eq!(values["groups.suggest.hard"], 85);
    assert_eq!(values["groups.suggest.modularity"], 0.3);
    assert_eq!(values["groups.suggest.jaccard"], 0.4);
    assert_eq!(values["groups.suggest.cooldown_days"], 7);
    assert_eq!(values["groups.suggest.decline_cooldown_days"], 14);
    let curve = values["groups.curve"].as_array().unwrap();
    assert_eq!(curve.len(), 9);
    assert_eq!(curve[0]["lower"], 3);
    assert_eq!(curve[0]["leave_down"], 1);
    assert_eq!(curve[3]["history_seconds"], 14 * 86_400);
    assert_eq!(curve[5]["vouches"], 2);
    assert!(curve[6]["joins_per_hour"].is_null());
    assert_eq!(curve[8]["joins_per_hour"], 6);
    // Dropping the removed fork-cost alias stays an open question.
    assert!(!values.contains_key("seats.fork_cost"));
}
