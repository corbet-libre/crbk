//! The platform catalogue defines cleanly and enforces authority and bounds.

use crbk::{Layer, PlatformValue, Rulebook, define_platform_settings, platform_catalog};
use serde_json::json;

fn book() -> Rulebook {
    let mut book = Rulebook::default();
    define_platform_settings(&mut book).unwrap();
    book
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn every_entry_defines_and_defaults_resolve() {
    let book = book();
    book.validate().unwrap();
    let all = platform_catalog();
    assert!(all.len() == 47);
    let resolved = book.resolve(&Default::default()).unwrap();
    for entry in &all {
        assert_eq!(resolved[&entry.key], entry.setting.default, "{}", entry.key);
        assert!(!entry.meta.label.is_empty() && !entry.meta.help.is_empty());
    }
    // Defining again changes nothing.
    let mut again = book.clone();
    define_platform_settings(&mut again).unwrap();
    assert_eq!(again, book);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn lowest_layers_bind_community_and_platform_only_values() {
    let mut book = book();
    // Voucher slopes and creation limits belong to Root alone.
    for key in [
        "voucher.slope.admin",
        "voucher.min.member",
        "community.create.per_week",
    ] {
        assert!(book.set_community(key, Some(json!(1))).is_err(), "{key}");
        book.set_platform(
            key,
            Some(PlatformValue {
                value: json!(2),
                force: false,
            }),
        )
        .unwrap();
    }
    // Success days: community may tighten above the floor, never below it.
    book.set_community("voucher.success.min_days", Some(json!(45)))
        .unwrap();
    assert!(
        book.set_community("voucher.success.min_days", Some(json!(13)))
            .is_err()
    );
    assert!(
        book.set_community("voucher.success.max_punishments", Some(json!(2)))
            .is_err()
    );
    // Wave waiting period 1 to 30.
    assert!(
        book.set_community("waves.waiting_period_days", Some(json!(0)))
            .is_err()
    );
    assert!(
        book.set_community("waves.waiting_period_days", Some(json!(31)))
            .is_err()
    );
    book.set_community("waves.waiting_period_days", Some(json!(30)))
        .unwrap();
    // Standing parameters keep the czkp safe ranges.
    assert!(
        book.set_community("standing.n_half", Some(json!(4)))
            .is_err()
    );
    book.set_community("standing.n_half", Some(json!(5)))
        .unwrap();
    // Optional success blocks accept null (off) and values.
    book.set_community("voucher.success.min_active_weeks", Some(json!(4)))
        .unwrap();
    book.set_community("voucher.success.min_active_weeks", Some(json!(null)))
        .unwrap();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn force_beats_the_community_and_describe_reports_every_layer() {
    let mut book = book();
    book.set_community("voucher.sponsor_visible", Some(json!(false)))
        .unwrap();
    book.set_platform(
        "voucher.sponsor_visible",
        Some(PlatformValue {
            value: json!(true),
            force: true,
        }),
    )
    .unwrap();
    let views = book.describe().unwrap();
    let view = views
        .iter()
        .find(|v| v.key == "voucher.sponsor_visible")
        .unwrap();
    assert_eq!(view.effective, json!(true));
    assert_eq!(view.community, Some(json!(false)));
    assert!(view.platform.as_ref().unwrap().force);
    assert_eq!(view.lowest_layer, Layer::Community);
    assert!(view.meta.one_way);
    // Screen order follows the catalogue; platform-only keys are reported with their layer.
    let slope = views
        .iter()
        .find(|v| v.key == "voucher.slope.root")
        .unwrap();
    assert_eq!(slope.lowest_layer, Layer::Platform);
    assert_eq!(views[0].key, "voucher.slope.member");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn role_bounds_are_consistent() {
    let book = book().resolve(&Default::default()).unwrap();
    for role in ["member", "admin", "root"] {
        let min = book[&format!("voucher.min.{role}")].as_i64().unwrap();
        let max = book[&format!("voucher.max.{role}")].as_i64().unwrap();
        assert!(min <= max, "{role}");
    }
}
