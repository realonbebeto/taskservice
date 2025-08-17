//! tests/health_check.rs
// `actix_web::test` is the testing equivalent of `tokio::main`

use crate::common;

#[cfg(test)]
mod tests {
    use super::common::spawn_app;

    #[actix_web::test]
    async fn health_check_works() {
        // Arrange
        let mut app = spawn_app().await;
        let client = reqwest::Client::new();

        // Act
        let response = client
            .get(&format!("{}/health_check", app.address))
            .send()
            .await
            .expect("failed to execute request");

        //Assert
        assert!(response.status().is_success());
        assert_eq!(Some(0), response.content_length());

        app.drop_test_db().await;
    }
}
