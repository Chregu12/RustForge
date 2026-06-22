//! Compile-and-behaviour test for the `#[model]` attribute macro.
//!
//! `#[model]` expands to a SeaORM entity module (`struct Model` +
//! `Relation` + `ActiveModelBehavior`) re-exported under the given name, plus
//! an `rf_db_facade::Model` impl. If this test crate compiles, the macro
//! produced valid code; the two models also prove their generated `Relation`
//! enums don't collide.

use rf_model_macro::model;

#[model]
pub struct Widget {
    pub name: String,
    #[hidden]
    pub secret: String,
}

#[model]
pub struct GadgetThing {
    pub label: String,
}

#[test]
fn models_expand_and_implement_the_model_trait() {
    // The `rf_db_facade::Model` impl carries the pluralized table name.
    assert_eq!(<widget::Model as rf_db_facade::Model>::TABLE, "widgets");
    assert_eq!(
        <gadget_thing::Model as rf_db_facade::Model>::TABLE,
        "gadget_things"
    );
}

#[test]
fn user_facing_aliases_exist() {
    // `Widget` is the record type (alias of `widget::Model`); the macro also
    // adds the standard id/created_at/updated_at columns.
    let epoch = chrono::DateTime::from_timestamp(0, 0).unwrap();
    let w = Widget {
        id: 7,
        name: "gear".to_string(),
        secret: "hidden".to_string(),
        created_at: epoch,
        updated_at: epoch,
    };
    assert_eq!(w.id, 7);
    assert_eq!(w.name, "gear");
    // `#[hidden]` becomes `#[serde(skip_serializing)]`: secret is omitted.
    let json = serde_json::to_string(&w).unwrap();
    assert!(json.contains("gear"));
    assert!(!json.contains("hidden"), "hidden field must not serialize: {json}");
}
