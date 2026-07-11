// Integration probe: tenancy
// Adapted from sandbox/probes/tenancy/src/main.rs
// Exercises rf-tenancy: multi-tenant scope, guard, spawn_with_tenant.

use axum::{body::Body, http::Request, routing::get, Router};
use rf_tenancy::{
    guard_tenant, scope_to_current, spawn_with_tenant, with_current_tenant,
    InMemoryTenantResolver, Tenant, TenantError, TenantLayer, TenantScoped,
};
use tower::ServiceExt; // oneshot

#[derive(Clone, Debug)]
struct Post {
    tenant_id: String,
    title: String,
}

impl TenantScoped for Post {
    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
}

fn all_posts() -> Vec<Post> {
    vec![
        Post { tenant_id: "acme".into(), title: "Acme welcome".into() },
        Post { tenant_id: "globex".into(), title: "Globex secret".into() },
        Post { tenant_id: "acme".into(), title: "Acme roadmap".into() },
    ]
}

async fn my_posts() -> String {
    let mine = scope_to_current(all_posts());
    let titles: Vec<String> = mine.into_iter().map(|p| p.title).collect();
    titles.join(",")
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    String::from_utf8_lossy(&bytes).to_string()
}

// multi_thread is required: spawn_with_tenant spawns tokio tasks.
#[tokio::test(flavor = "multi_thread")]
async fn test_tenancy() -> anyhow::Result<()> {
    let resolver = InMemoryTenantResolver::new();
    resolver.add_tenant(Tenant::new("acme", "Acme Inc")).await;
    resolver.add_tenant(Tenant::new("globex", "Globex")).await;

    let app = Router::new()
        .route("/posts", get(my_posts))
        .layer(TenantLayer::by_header("X-Tenant-Id", resolver));

    // Tenant ACME sees only acme rows.
    let acme_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/posts")
                .header("X-Tenant-Id", "acme")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let acme_body = body_string(acme_resp).await;
    assert_eq!(acme_body, "Acme welcome,Acme roadmap");
    assert!(!acme_body.contains("Globex"));

    // Tenant GLOBEX sees only globex rows.
    let globex_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/posts")
                .header("X-Tenant-Id", "globex")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let globex_body = body_string(globex_resp).await;
    assert_eq!(globex_body, "Globex secret");
    assert!(!globex_body.contains("Acme"));

    // Unknown tenant header -> 404.
    let unknown_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/posts")
                .header("X-Tenant-Id", "does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown_resp.status(), axum::http::StatusCode::NOT_FOUND);

    // Missing X-Tenant-Id header -> 400 Bad Request (NOT 500).
    let missing_header_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/posts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_header_resp.status(), axum::http::StatusCode::BAD_REQUEST);

    // Outside any request scope, there is no current tenant.
    assert!(Tenant::current().is_none());

    // guard_tenant: own row passes, foreign row yields CrossTenantAccess.
    with_current_tenant(Tenant::new("acme", "Acme Inc"), async {
        let own = Post { tenant_id: "acme".into(), title: "ok".into() };
        assert!(guard_tenant(&own).is_ok());

        let foreign = Post { tenant_id: "globex".into(), title: "leak".into() };
        match guard_tenant(&foreign) {
            Err(TenantError::CrossTenantAccess) => {}
            other => panic!("expected CrossTenantAccess, got {other:?}"),
        }
    })
    .await;

    // spawn_with_tenant carries the current tenant into a spawned task.
    with_current_tenant(Tenant::new("acme", "Acme Inc"), async {
        // Plain tokio::spawn loses the task-local.
        let plain_id = tokio::spawn(async { Tenant::current_id() }).await.unwrap();
        assert!(plain_id.is_none());

        // spawn_with_tenant re-establishes the tenant scope.
        let carrying_id = spawn_with_tenant(async { Tenant::current_id() })
            .await
            .unwrap();
        assert_eq!(carrying_id.as_deref(), Some("acme"));
    })
    .await;

    Ok(())
}
