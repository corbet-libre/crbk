//! Sparse resolution, validation and template pinning contract.

mod common;

use common::*;
use crbk::*;
use serde_json::{Value, json};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sparse_chain_and_force_exhaustively_preserve_null() {
    let choices = [None, Some(Value::Null), Some(json!(30))];
    for p in &choices {
        for t in &choices {
            for c in &choices {
                for m in &choices {
                    for force in [false, true] {
                        let mut b = book();
                        b.set_platform(
                            "quota",
                            p.clone().map(|value| PlatformValue { value, force }),
                        )
                        .unwrap();
                        b.publish_template(
                            pin(1),
                            t.clone()
                                .map(|v| [("quota".into(), v)].into())
                                .unwrap_or_default(),
                        )
                        .unwrap();
                        b.pin_template(Some(pin(1))).unwrap();
                        b.set_community("quota", c.clone()).unwrap();
                        let member = m
                            .clone()
                            .map(|v| [("quota".into(), v)].into())
                            .unwrap_or_default();
                        let expected = if force && p.is_some() {
                            p.clone()
                        } else {
                            m.clone().or(c.clone()).or(t.clone()).or(p.clone())
                        }
                        .unwrap_or(json!(10));
                        assert_eq!(b.resolve(&member).unwrap()["quota"], expected);
                    }
                }
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn pinned_templates_and_live_technical_values() {
    let mut b = book();
    b.set_platform("quota", platform(json!(20), false)).unwrap();
    b.publish_template(pin(1), Values::new()).unwrap();
    b.pin_template(Some(pin(1))).unwrap();
    b.set_platform("quota", platform(json!(40), false)).unwrap();
    b.set_platform("timeout", platform(json!(40), false))
        .unwrap();
    let values = b.resolve(&Values::new()).unwrap();
    assert_eq!(values["quota"], 20);
    assert_eq!(values["timeout"], 40);
    b.publish_template(pin(2), Values::new()).unwrap();
    assert_eq!(b.resolve(&Values::new()).unwrap()["quota"], 20);
    b.pin_template(Some(pin(2))).unwrap();
    assert_eq!(b.resolve(&Values::new()).unwrap()["quota"], 40);
    b.set_platform("quota", platform(Value::Null, true))
        .unwrap();
    assert_eq!(b.resolve(&Values::new()).unwrap()["quota"], Value::Null);
    b.set_platform("quota", None).unwrap();
    assert_eq!(b.resolve(&Values::new()).unwrap()["quota"], 40);
    assert!(b.publish_template(pin(2), Values::new()).is_err());
    assert!(b.pin_template(Some(pin(99))).is_err());
    assert!(
        b.publish_template(pin(3), [("timeout".into(), json!(1))].into())
            .is_err()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn every_layer_enforces_types_bounds_and_authority_even_when_shadowed() {
    let mut b = book();
    for bad in [
        json!(-1),
        json!(101),
        json!(1.5),
        json!("secret"),
        json!(true),
        json!([]),
    ] {
        assert!(
            b.set_platform("quota", platform(bad.clone(), true))
                .is_err()
        );
        assert!(b.set_community("quota", Some(bad.clone())).is_err());
        assert!(
            b.publish_template(pin(1), [("quota".into(), bad.clone())].into())
                .is_err()
        );
        assert!(b.resolve(&[("quota".into(), bad)].into()).is_err());
    }
    b.set_platform("quota", platform(json!(20), true)).unwrap();
    assert!(b.resolve(&[("quota".into(), json!(101))].into()).is_err());
    assert!(b.resolve(&[("timeout".into(), json!(20))].into()).is_err());
    assert!(b.set_community("unknown", None).is_err());
    assert!(b.set_platform("unknown", None).is_err());
    let mut s = setting(SettingKind::Template, Layer::Platform);
    s.nullable = false;
    assert!(s.check(&Value::Null).is_err());
    b.define("locked", s).unwrap();
    assert!(b.set_community("locked", Some(json!(1))).is_err());
    assert!(
        b.publish_template(pin(1), [("locked".into(), json!(1))].into())
            .is_err()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exact_large_integer_bounds_and_invalid_catalogue() {
    let mut b = Rulebook::default();
    let mut s = setting(SettingKind::Technical, Layer::Member);
    s.default = json!(u64::MAX);
    s.bounds = Bounds {
        min: Some(u64::MAX.into()),
        max: Some(u64::MAX.into()),
    };
    b.define("large", s.clone()).unwrap();
    assert!(b.set_community("large", Some(json!(u64::MAX - 1))).is_err());
    assert!(b.set_community("large", Some(json!(u64::MAX))).is_ok());
    s.bounds.min = Some(serde_json::Number::from_f64(0.5).unwrap());
    assert!(b.define("float_bound", s).is_err());
    let mut s = setting(SettingKind::Technical, Layer::Member);
    s.bounds = Bounds {
        min: Some(20.into()),
        max: Some(10.into()),
    };
    assert!(b.define("reversed", s).is_err());
    let mut s = setting(SettingKind::Technical, Layer::Member);
    s.default = json!(101);
    assert!(b.define("bad_default", s).is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn whole_objects_replace_and_deserialization_cannot_bypass_validation() {
    let mut b = Rulebook::default();
    b.define(
        "object",
        Setting {
            value_type: SettingType::Object,
            nullable: true,
            default: json!({"one":1,"two":2}),
            bounds: Bounds::default(),
            lowest_layer: Layer::Member,
            kind: SettingKind::Technical,
        },
    )
    .unwrap();
    b.set_community("object", Some(json!({"one":3}))).unwrap();
    assert_eq!(
        b.resolve(&Values::new()).unwrap()["object"],
        json!({"one":3})
    );
    let mut encoded = serde_json::to_value(b).unwrap();
    encoded["community"]["object"] = json!("private-input");
    let b: Rulebook = serde_json::from_value(encoded).unwrap();
    let error = b.resolve(&Values::new()).unwrap_err();
    assert!(!format!("{error:?} {error}").contains("private-input"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn control_settings_cannot_be_member_overridden() {
    let mut b = Rulebook::default();
    let s = Setting {
        value_type: SettingType::Boolean,
        nullable: true,
        default: json!(true),
        bounds: Bounds::default(),
        lowest_layer: Layer::Member,
        kind: SettingKind::Technical,
    };
    assert!(
        b.define(gate_key(GateLevel::Global, "phone"), s.clone())
            .is_err()
    );
    assert!(b.define("gate/invalid/phone", s.clone()).is_err());
    assert!(b.define("action/enter", s).is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn direct_checks_reject_malformed_bounds_even_for_nullable_values() {
    for (kind, min, max) in [
        (SettingType::Integer, "0.5", "100"),
        (SettingType::Integer, "20", "10"),
        (SettingType::Number, "1e400", "1e401"),
        (SettingType::Number, "-1e400", "10"),
        (SettingType::Boolean, "0", "1"),
    ] {
        let mut s = setting(SettingKind::Technical, Layer::Member);
        s.value_type = kind;
        s.bounds = Bounds {
            min: Some(min.parse().unwrap()),
            max: Some(max.parse().unwrap()),
        };
        assert!(s.check(&json!(10)).is_err());
        assert!(s.check(&Value::Null).is_err());
        assert!(Rulebook::default().define("invalid", s).is_err());
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn forced_numbers_reject_nonfinite_values_and_restore_deviations() {
    let mut s = setting(SettingKind::Technical, Layer::Member);
    s.value_type = SettingType::Number;
    s.bounds = Bounds::default();
    let mut b = Rulebook::default();
    b.define("number", s.clone()).unwrap();
    b.set_community("number", Some(json!(25))).unwrap();
    b.set_platform("number", platform(json!(50), true)).unwrap();
    let huge: Value = serde_json::from_str("1e400").unwrap();
    assert!(s.check(&huge).is_err());
    assert!(
        b.set_platform("number", platform(huge.clone(), true))
            .is_err()
    );
    assert!(b.resolve(&[("number".into(), huge)].into()).is_err());
    assert_eq!(b.resolve(&Values::new()).unwrap()["number"], json!(50));
    b.set_platform("number", platform(json!(50), false))
        .unwrap();
    assert_eq!(b.resolve(&Values::new()).unwrap()["number"], json!(25));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn identifiers_duplicate_definitions_and_numeric_bound_edges_fail_closed() {
    let base = setting(SettingKind::Technical, Layer::Member);
    for key in [String::new(), "x".repeat(201), "has space".into()] {
        assert!(Rulebook::default().define(key, base.clone()).is_err());
    }
    let mut book = book();
    assert!(book.define("quota", base.clone()).is_err());
    book.pin_template(None).unwrap();
    assert!(book.publish_template(pin(0), Values::new()).is_err());
    let mut number = base;
    number.value_type = SettingType::Number;
    number.bounds = Bounds {
        min: Some(1.into()),
        max: Some(2.into()),
    };
    assert!(number.check(&json!(1.0)).is_ok());
    assert!(number.check(&json!(2.0)).is_ok());
    assert!(number.check(&json!(0.5)).is_err());
    assert!(number.check(&json!(2.5)).is_err());
    number.bounds.min = None;
    assert!(number.check(&json!(-10.5)).is_ok());
    number.bounds.max = None;
    assert!(number.check(&json!(10.5)).is_ok());
    number.bounds.min = Some(2.into());
    number.bounds.max = Some(1.into());
    assert!(number.check(&json!(1.5)).is_err());
    number.bounds.min = Some(0.into());
    number.bounds.max = Some("1e400".parse().unwrap());
    assert!(number.check(&json!(1.0)).is_err());
    for (kind, value) in [(SettingType::String, json!("value")), (SettingType::Array, json!([1]))] {
        let mut definition = setting(SettingKind::Technical, Layer::Member);
        definition.value_type = kind;
        definition.bounds = Bounds::default();
        assert!(definition.check(&value).is_ok());
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn deserialized_template_versions_and_pins_are_validated() {
    let mut book = book();
    book.publish_template(pin(1), Values::new()).unwrap();
    let encoded = serde_json::to_value(&book).unwrap();
    let mut zero = encoded.clone();
    let template = zero["templates"]["standard"]["1"].take();
    zero["templates"]["standard"] = json!({"0": template});
    let invalid: Rulebook = serde_json::from_value(zero).unwrap();
    assert!(invalid.validate().is_err());
    let mut missing = encoded;
    missing["pin"] = serde_json::to_value(pin(2)).unwrap();
    let invalid: Rulebook = serde_json::from_value(missing).unwrap();
    assert!(invalid.resolve(&Values::new()).is_err());
}
