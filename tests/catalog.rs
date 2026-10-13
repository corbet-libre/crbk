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
    assert!(all.len() == 48);
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

/// The thirty-two keys `czkp::standing::Params::from_rulebook` reads, with the
/// catalogue values that make the reference community. czkp has no defaults; cvld
/// pins the digest of exactly these numbers against the circuit.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn the_standing_and_voucher_defaults_are_the_reference_community() {
    let resolved = book().resolve(&Default::default()).unwrap();
    for (key, expected) in [
        ("standing.c0", json!(3)),
        ("standing.h_a", json!(14)),
        ("standing.h_g", json!(14)),
        ("standing.n_half", json!(25)),
        ("standing.n_c", json!(60)),
        ("standing.e_q", json!(16_384)),
        ("standing.r_max", json!(40)),
        ("standing.r_age", json!(2)),
        ("standing.c_max_q", json!(52_428)),
        ("standing.c_vet", json!(30)),
        ("standing.age_vet", json!(60)),
        ("standing.boost_q", json!(65_536)),
        ("standing.boost_days", json!(30)),
        ("standing.slope_q", json!(262_144)),
        ("standing.mid_q", json!(49_152)),
        ("standing.kappa_q", json!(131_072)),
        ("standing.loan_use_q", json!(16_384)),
        ("voucher.slope.member", json!(0.1)),
        ("voucher.slope.admin", json!(2.0)),
        ("voucher.slope.root", json!(25.0)),
        ("voucher.min.member", json!(1)),
        ("voucher.min.admin", json!(50)),
        ("voucher.min.root", json!(500)),
        ("voucher.max.member", json!(3)),
        ("voucher.max.admin", json!(100)),
        ("voucher.max.root", json!(1000)),
        ("voucher.success.min_days", json!(30)),
        ("voucher.success.min_conversations", json!(3)),
        ("voucher.success.max_punishments", json!(1)),
        ("voucher.brake_cap", json!(1)),
        ("voucher.reward.enabled", json!(true)),
        ("voucher.reward.cap", json!(2)),
    ] {
        assert_eq!(resolved[key], expected, "{key}");
    }
    // Root alone sets the brake threshold.
    let mut book = book();
    assert!(book.set_community("voucher.brake_cap", Some(json!(2))).is_err());
}
