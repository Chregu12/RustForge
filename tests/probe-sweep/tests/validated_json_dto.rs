// Integration probe: validated_json_dto
// Adapted from sandbox/probes/validated_json_dto/src/main.rs
// Proves the end-to-end typed-DTO-validation path via ValidatedJson<T>.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::post,
    Router,
};
use rf::Model;
use rf_db_facade::Model as ModelTrait;
use rf_validation::ValidatedJson;
use tower::ServiceExt; // oneshot

Model!(Article {
    validated,
    title: String @ min(1) max(20),
    email: String @ email message("Please enter a valid email"),
    website: String @ url,
    token: String @ uuid,
    client_ip: String @ ip,
    zipcode: String @ regex("^\\d{5}$"),
    slug: String @ alpha,
    username: String @ alphanumeric,
    code: String @ starts_with("SKU-"),
    filename: String @ ends_with(".txt"),
    rating: i64 @ range(1, 5),
    views: i64,
    body: String,
});

const VALID_UUID: &str = "550e8400-e29b-41d4-a716-446655440000";

fn valid_body() -> String {
    format!(
        r#"{{"title":"Hello","email":"a@b.com","website":"https://example.com","token":"{}","client_ip":"192.168.1.1","zipcode":"12345","slug":"hello","username":"user42","code":"SKU-123","filename":"report.txt","rating":3,"views":42,"body":"x"}}"#,
        VALID_UUID
    )
}

async fn create_article(ValidatedJson(article): ValidatedJson<CreateArticle>) -> String {
    format!("created: {} ({} views)", article.title, article.views)
}

fn app() -> Router {
    Router::new().route("/articles", post(create_article))
}

async fn post_json(body: &str) -> StatusCode {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/articles")
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

#[tokio::test]
async fn test_validated_json_dto() {
    // Entity + Model trait intact.
    assert_eq!(<Article as ModelTrait>::TABLE, "articles");

    // CreateArticle/UpdateArticle implement rf_validation::Validate.
    fn assert_is_validate<T: rf_validation::Validate>() {}
    assert_is_validate::<CreateArticle>();
    assert_is_validate::<UpdateArticle>();

    // Valid body -> 200 OK.
    assert_eq!(post_json(&valid_body()).await, StatusCode::OK);

    // Missing required field -> 400.
    assert_eq!(
        post_json(r#"{"email":"a@b.com","views":42,"body":"x"}"#).await,
        StatusCode::BAD_REQUEST
    );

    // Wrong type -> 400.
    assert_eq!(
        post_json(r#"{"title":"Hi","email":"a@b.com","views":"not-a-number","body":"x"}"#).await,
        StatusCode::BAD_REQUEST
    );

    // Empty required string -> 422.
    assert_eq!(
        post_json(&valid_body().replace(r#""title":"Hello""#, r#""title":"""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Title too long (@ max(20)) -> 422.
    let long_title = "x".repeat(50);
    assert_eq!(
        post_json(&valid_body().replace(r#""title":"Hello""#, &format!(r#""title":"{}""#, long_title))).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Invalid email (@ email) -> 422.
    assert_eq!(
        post_json(&valid_body().replace(r#""email":"a@b.com""#, r#""email":"not-an-email""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Invalid url (@ url) -> 422.
    assert_eq!(
        post_json(&valid_body().replace(r#""website":"https://example.com""#, r#""website":"not a url""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Invalid uuid (@ uuid) -> 422.
    assert_eq!(
        post_json(&valid_body().replace(&format!(r#""token":"{}""#, VALID_UUID), r#""token":"not-a-uuid""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Invalid ip (@ ip) -> 422.
    assert_eq!(
        post_json(&valid_body().replace(r#""client_ip":"192.168.1.1""#, r#""client_ip":"999.999.999.999""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Valid IPv6 (@ ip accepts v4 OR v6) -> 200.
    assert_eq!(
        post_json(&valid_body().replace(r#""client_ip":"192.168.1.1""#, r#""client_ip":"2001:db8::8a2e:370:7334""#)).await,
        StatusCode::OK
    );

    // Out-of-range rating (@ range(1,5)) -> 422.
    assert_eq!(
        post_json(&valid_body().replace(r#""rating":3"#, r#""rating":9"#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // In-range boundaries -> 200.
    for r in [1i64, 5] {
        assert_eq!(
            post_json(&valid_body().replace(r#""rating":3"#, &format!(r#""rating":{}"#, r))).await,
            StatusCode::OK
        );
    }

    // Non-matching zipcode (@ regex) -> 422.
    assert_eq!(
        post_json(&valid_body().replace(r#""zipcode":"12345""#, r#""zipcode":"abcde""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Matching zipcode -> 200.
    assert_eq!(
        post_json(&valid_body().replace(r#""zipcode":"12345""#, r#""zipcode":"90210""#)).await,
        StatusCode::OK
    );

    // @ alpha: digit -> 422; letters -> 200.
    assert_eq!(
        post_json(&valid_body().replace(r#""slug":"hello""#, r#""slug":"hel10""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post_json(&valid_body().replace(r#""slug":"hello""#, r#""slug":"World""#)).await,
        StatusCode::OK
    );

    // @ alphanumeric: underscore -> 422; letters+digits -> 200.
    assert_eq!(
        post_json(&valid_body().replace(r#""username":"user42""#, r#""username":"user_42""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post_json(&valid_body().replace(r#""username":"user42""#, r#""username":"abc123""#)).await,
        StatusCode::OK
    );

    // @ starts_with: wrong prefix -> 422; correct prefix -> 200.
    assert_eq!(
        post_json(&valid_body().replace(r#""code":"SKU-123""#, r#""code":"XYZ-123""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post_json(&valid_body().replace(r#""code":"SKU-123""#, r#""code":"SKU-777""#)).await,
        StatusCode::OK
    );

    // @ ends_with: wrong suffix -> 422; correct suffix -> 200.
    assert_eq!(
        post_json(&valid_body().replace(r#""filename":"report.txt""#, r#""filename":"report.md""#)).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post_json(&valid_body().replace(r#""filename":"report.txt""#, r#""filename":"data.txt""#)).await,
        StatusCode::OK
    );

    // Direct Validate call confirms it's the real trait.
    let bad = CreateArticle {
        title: String::new(),
        email: "nope".to_string(),
        website: "nope".to_string(),
        token: "nope".to_string(),
        client_ip: "nope".to_string(),
        zipcode: "abc".to_string(),
        slug: "ab1".to_string(),
        username: "us er".to_string(),
        code: "XYZ-1".to_string(),
        filename: "report.md".to_string(),
        rating: 42,
        views: 1,
        body: "b".to_string(),
    };
    let err = rf_validation::Validate::validate(&bad).expect_err("bad DTO must fail");
    let fields = err.field_errors();
    assert!(fields.contains_key("title"));
    assert!(fields.contains_key("email"));
    assert_eq!(fields["email"][0].code, "email");
    assert_eq!(
        fields["email"][0].message.as_deref(),
        Some("Please enter a valid email")
    );
    assert!(fields.contains_key("website"));
    assert_eq!(
        fields["website"][0].message.as_deref(),
        Some("The website field must be a valid URL.")
    );
    assert_eq!(fields["token"][0].code, "uuid");
    assert_eq!(fields["client_ip"][0].code, "ip");
    assert_eq!(fields["zipcode"][0].code, "regex");
    assert_eq!(fields["slug"][0].code, "alpha");
    assert_eq!(fields["username"][0].code, "alpha_numeric");
    assert_eq!(fields["code"][0].code, "starts_with");
    assert_eq!(fields["filename"][0].code, "ends_with");

    let good = CreateArticle {
        title: "Fine".to_string(),
        email: "a@b.com".to_string(),
        website: "https://example.com".to_string(),
        token: VALID_UUID.to_string(),
        client_ip: "10.0.0.1".to_string(),
        zipcode: "90210".to_string(),
        slug: "hello".to_string(),
        username: "user42".to_string(),
        code: "SKU-999".to_string(),
        filename: "notes.txt".to_string(),
        rating: 4,
        views: 1,
        body: "b".to_string(),
    };
    assert!(rf_validation::Validate::validate(&good).is_ok());

    // Update DTO path: empty UpdateArticle validates fine.
    let empty_update = UpdateArticle::default();
    assert!(rf_validation::Validate::validate(&empty_update).is_ok());

    // Present-but-invalid uuid/ip on Update DTO must fail.
    let mut bad_update = UpdateArticle::default();
    bad_update.token = Some("not-a-uuid".to_string());
    bad_update.client_ip = Some("999.1.1.1".to_string());
    bad_update.zipcode = Some("bad".to_string());
    bad_update.slug = Some("ab1".to_string());
    bad_update.code = Some("XYZ-1".to_string());
    bad_update.filename = Some("a.md".to_string());
    let uerr = rf_validation::Validate::validate(&bad_update)
        .expect_err("present invalid uuid/ip on Update DTO must fail");
    let ufields = uerr.field_errors();
    assert!(ufields.contains_key("token"));
    assert!(ufields.contains_key("client_ip"));
    assert_eq!(ufields["zipcode"][0].code, "regex");
    assert_eq!(ufields["slug"][0].code, "alpha");
    assert_eq!(ufields["code"][0].code, "starts_with");
    assert_eq!(ufields["filename"][0].code, "ends_with");
}
