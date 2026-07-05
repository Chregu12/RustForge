//! Proves `#[relations]` PRESERVES method bodies (previously discarded) and
//! emits a `<method>_kind()` introspection companion.

use rf_model_macro::relations;
use std::marker::PhantomData;

struct Post;
struct Profile;

// Local stand-ins for relationship builder types; the macro recognizes them by
// the textual return type (HasMany / HasOne / BelongsTo).
struct HasMany<T>(PhantomData<T>);
struct HasOne<T>(PhantomData<T>);

struct User {
    id: i32,
}

#[relations]
impl User {
    // The body MUST be preserved and remain callable.
    fn posts(&self) -> HasMany<Post> {
        assert_eq!(self.id, 7, "self is available inside the preserved body");
        HasMany(PhantomData)
    }

    fn profile(&self) -> HasOne<Profile> {
        let _ = self.id;
        HasOne(PhantomData)
    }
}

#[test]
fn relations_preserves_method_bodies() {
    let u = User { id: 7 };
    // If the body were discarded, these methods would not exist / would not run.
    let _posts: HasMany<Post> = u.posts();
    let _profile: HasOne<Profile> = u.profile();
}

#[test]
fn relations_emits_kind_companions() {
    assert_eq!(User::posts_kind(), "HasMany");
    assert_eq!(User::profile_kind(), "HasOne");
}
