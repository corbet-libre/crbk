#![allow(dead_code)]

use crbk::*;
use serde_json::json;

pub fn setting(kind: SettingKind, lowest_layer: Layer) -> Setting {
    Setting {
        value_type: SettingType::Integer,
        nullable: true,
        default: json!(10),
        bounds: Bounds {
            min: Some(0.into()),
            max: Some(100.into()),
        },
        lowest_layer,
        kind,
    }
}

pub fn book() -> Rulebook {
    let mut book = Rulebook::default();
    book.define("quota", setting(SettingKind::Template, Layer::Member))
        .unwrap();
    book.define("timeout", setting(SettingKind::Technical, Layer::Community))
        .unwrap();
    book
}

pub fn platform(value: serde_json::Value, force: bool) -> Option<PlatformValue> {
    Some(PlatformValue { value, force })
}

pub fn pin(version: u64) -> TemplatePin {
    TemplatePin {
        name: "standard".into(),
        version,
    }
}

pub fn change(rulebook: Rulebook, announced_at: i64, effective_at: i64) -> Change {
    Change {
        rulebook,
        announced_at,
        effective_at,
        notice_seconds: 0,
        policy_epoch: 1,
    }
}

pub fn requirement(gate: &str) -> Requirement {
    Requirement {
        gate: gate.into(),
        level: GateLevel::Community,
        provider: None,
    }
}

pub fn proof(gate: &str) -> GateResult {
    GateResult {
        gate: gate.into(),
        level: GateLevel::Community,
        subject: "member".into(),
        community: Some("alpha".into()),
        provider: "local".into(),
        valid_until: 200,
        proven_at: Some(90),
    }
}

pub fn subject() -> Subject<'static> {
    Subject {
        id: "member",
        membership: MembershipState::Admitted,
    }
}

pub fn decision_book(policy: ActionPolicy) -> Rulebook {
    let mut book = book();
    for gate in ["voucher", "phone", "payment"] {
        for key in [
            gate_key(GateLevel::Community, gate),
            provider_key(GateLevel::Community, gate, "local"),
        ] {
            book.define(
                key,
                Setting {
                    value_type: SettingType::Boolean,
                    nullable: true,
                    default: json!(true),
                    bounds: Bounds::default(),
                    lowest_layer: Layer::Community,
                    kind: SettingKind::Technical,
                },
            )
            .unwrap();
        }
    }
    book.define(
        action_key("enter"),
        Setting {
            value_type: SettingType::Policy,
            nullable: true,
            default: serde_json::to_value(policy).unwrap(),
            bounds: Bounds::default(),
            lowest_layer: Layer::Community,
            kind: SettingKind::Technical,
        },
    )
    .unwrap();
    book
}

pub fn snapshot(book: Rulebook) -> Snapshot {
    Revision {
        revision: 1,
        change: change(book, 0, 0),
    }
    .snapshot("alpha", 100)
    .unwrap()
}

#[cfg(all(feature = "sql", not(target_arch = "wasm32")))]
pub async fn local() -> (tempfile::TempDir, crlt::Db, LibsqlStore) {
    let dir = tempfile::tempdir().unwrap();
    let config = crlt::Config::new(
        format!("file://{}", dir.path().join("rules.db").display()),
        "",
    );
    let db = crlt::Db::open(config).await.unwrap();
    db.migrate(&[crlt::Migration::new(1, "rulebook", SCHEMA)])
        .await
        .unwrap();
    let store = LibsqlStore::new(db.clone());
    (dir, db, store)
}
