/// Feature Tests
///
/// Feature tests test larger portions of your application,
/// including how multiple components interact with each other.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_homepage_loads() {
        // Example feature test
        // let response = test_client()
        //     .get("/")
        //     .send()
        //     .await
        //     .unwrap();
        //
        // assert_eq!(response.status(), 200);
        // assert!(response.text().await.unwrap().contains("Welcome"));
    }

    #[tokio::test]
    async fn test_user_can_register() {
        // Example registration test
        // let response = test_client()
        //     .post("/register")
        //     .json(&json!({
        //         "name": "John Doe",
        //         "email": "john@example.com",
        //         "password": "password123"
        //     }))
        //     .send()
        //     .await
        //     .unwrap();
        //
        // assert_eq!(response.status(), 201);
    }
}
