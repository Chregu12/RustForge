use rf::prelude::*;

Model!(Post {
    timestamps = false,
    guarded,
    title: String,
    body: String,
});
