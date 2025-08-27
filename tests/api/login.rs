use crate::common;

mod tests {

    use taskservice::error::authentication::LoginResponse;

    use super::common::spawn_app;

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

        let response_body: LoginResponse = response.json().await.unwrap();
        assert!(response_body.message.contains("Authentication failed"));

        // Assert Par 2 - Session cookie is still there
        let response: LoginResponse = app.get_login().await.json().await.unwrap();
        assert!(response.message.contains("Authentication failed"));

        // Assert Part 3 - Session cookie has been updated
        let response: LoginResponse = app.get_login().await.json().await.unwrap();
        assert_eq!(response.message, "");

        app.drop_test_db().await;
    }
}
