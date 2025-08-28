use crate::common;

mod tests {
    use super::common::{StdResponse, spawn_app};

    #[actix_web::test]
    async fn logout_clears_session_state() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(),
                                                    "password": app.test_profile.password.as_ref()});
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

        // Log out - 1
        let response = app.post_logout().await;
        let response: StdResponse = response.json().await.unwrap();
        assert!(
            response
                .message
                .contains("You have successfully logged out.")
        );

        // Log out - 2
        let response = app.post_logout().await;
        let response: StdResponse = response.json().await.unwrap();
        assert!(response.message.contains("No active session"));

        app.drop_test_db().await;
    }
}
