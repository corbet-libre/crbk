//! Common frontier policy uses the real sparse resolver on native and Wasm.
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
fn common_epoch_defaults_overrides_and_layer_authority() {
    let mut book = Rulebook::default();
    assert!(RecordSettings::from_snapshot(&snapshot(&book)).is_err());
    define_record_settings(&mut book).unwrap();
    assert_eq!(
        RecordSettings::from_snapshot(&snapshot(&book))
            .unwrap()
            .epoch_seconds,
        86400
    );
    let original = book.clone();
    define_record_settings(&mut book).unwrap();
    assert_eq!(book, original);
    book.set_community(RECORD_EPOCH_SECONDS, Some(json!(3600)))
        .unwrap();
    assert_eq!(
        RecordSettings::from_snapshot(&snapshot(&book))
            .unwrap()
            .epoch_seconds,
        3600
    );
    book.set_platform(
        RECORD_EPOCH_SECONDS,
        Some(PlatformValue {
            value: json!(7200),
            force: true,
        }),
    )
    .unwrap();
    assert_eq!(
        RecordSettings::from_snapshot(&snapshot(&book))
            .unwrap()
            .epoch_seconds,
        7200
    );
    assert!(
        book.resolve(&Values::from([(RECORD_EPOCH_SECONDS.into(), json!(1))]))
            .is_err()
    );
    for invalid in [
        json!(null),
        json!(0),
        json!(-1),
        json!(1.5),
        json!("86400"),
        json!(1u64 << 53),
    ] {
        assert!(
            book.set_community(RECORD_EPOCH_SECONDS, Some(invalid.clone()))
                .is_err()
        );
        let mut untrusted = snapshot(&book);
        untrusted
            .content
            .insert(RECORD_EPOCH_SECONDS.into(), invalid);
        assert!(RecordSettings::from_snapshot(&untrusted).is_err());
    }
}
