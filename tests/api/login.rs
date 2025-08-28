use crate::common;

mod tests {

    use super::common::{StdResponse, spawn_app};

    #[actix_web::test]
    async fn an_error_flash_message_is_set_on_failure() {
        // Arrange
        let mut app = spawn_app().await;

        // Act Part 1 - Try to login
        // Cookie is set
        let login_body =
            serde_json::json!({"username": "random-username", "password": "random-password"});

        let response = app.post_login(&login_body).await;
        let status = response.status().as_u16();

        // Assert Part 1 - Login is Unauthorized with cookie
        assert_eq!(status, 401);

        let response_body: StdResponse = response.json().await.unwrap();
        assert!(response_body.message.contains("Authentication failed"));

        // Assert Par 2 - Session cookie is still there
        let response: StdResponse = app.get_login().await.json().await.unwrap();
        assert!(response.message.contains("Authentication failed"));

        // Assert Part 3 - Session cookie has been updated
        let response: StdResponse = app.get_login().await.json().await.unwrap();
        assert_eq!(response.message, "");

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn session_persists_and_extends_to_other_routes() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(), "password": app.test_profile.password.as_ref()});

        app.post_login(&login_body).await;
        let response = app
            .api_client
            .get(format!("{}/admin/dashboard", &app.address))
            .send()
            .await
            .expect("Failed to execute request");

        // Confirm if dragonfly a live session
        let response: StdResponse = response.json().await.unwrap();
        let pattern = format!("Welcome {}", app.test_profile.username.as_ref().to_string());
        assert!(response.message.contains(&pattern));

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn you_must_be_logged_in_to_access_admin_dashboard() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        let response = app
            .api_client
            .get(format!("{}/admin/dashboard", &app.address))
            .send()
            .await
            .expect("Failed to execute request");

        // Confirm if dragonfly a live session
        let response: StdResponse = response.json().await.unwrap();
        assert!(response.message.contains("Welcome Person of the Internet"));

        app.drop_test_db().await;
    }
}
