//! The wave waiting period uses the real sparse resolver and authority checks.

use crbk::*;
use serde_json::json;

fn snapshot(book: &Rulebook) -> Snapshot {
    Snapshot {
        community: "garden".into(),
        kind: SnapshotKind::Settings,
        revision: 1,
        policy_epoch: 1,
        issued: 0,
        content: book.resolve(&Values::new()).unwrap(),
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn waiting_period_defaults_to_seven_days_and_is_bounded_by_the_community() {
    let mut book = Rulebook::default();
    assert!(WaveSettings::from_snapshot(&snapshot(&book)).is_err());
    define_wave_settings(&mut book).unwrap();
    let initial = WaveSettings::from_snapshot(&snapshot(&book)).unwrap();
    assert_eq!(initial.waiting_period_days, 7);
    assert_eq!(initial.waiting_period_seconds(), 7 * 86_400);
    for ok in [1, 30] {
        book.set_community(WAVE_WAITING_PERIOD_DAYS, Some(json!(ok)))
            .unwrap();
        assert_eq!(
            WaveSettings::from_snapshot(&snapshot(&book))
                .unwrap()
                .waiting_period_days,
            ok
        );
    }
    for bad in [json!(null), json!(0), json!(31), json!(1.5), json!("7")] {
        assert!(
            book.set_community(WAVE_WAITING_PERIOD_DAYS, Some(bad))
                .is_err()
        );
    }
    assert!(
        book.resolve(&Values::from([(WAVE_WAITING_PERIOD_DAYS.into(), json!(3))]))
            .is_err()
    );
    let mut invalid = snapshot(&book);
    invalid
        .content
        .insert(WAVE_WAITING_PERIOD_DAYS.into(), json!(null));
    assert!(WaveSettings::from_snapshot(&invalid).is_err());
    invalid
        .content
        .insert(WAVE_WAITING_PERIOD_DAYS.into(), json!(31));
    assert!(WaveSettings::from_snapshot(&invalid).is_err());
    // Defining twice keeps the existing definition.
    define_wave_settings(&mut book).unwrap();
}
