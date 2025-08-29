use crate::common;

mod tests {
    use super::common::{StdResponse, spawn_app};
    use uuid::Uuid;

    #[actix_web::test]
    async fn no_change_password_access_when_logged_out() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        let new_password = Uuid::new_v4().to_string();
        let pass_body = serde_json::json!({"current_password": app.test_profile.password.as_ref(),
                                                    "new_password": &new_password,
                                                    "new_password_check": &new_password });

        let response = app.post_change_password(&pass_body).await;

        let response: StdResponse = response.json().await.unwrap();
        assert!(
            response
                .message
                .contains("You are not logged in. Please log in...")
        );

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn change_password_access_when_logged_in() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(),
                                                    "password": app.test_profile.password.as_ref()});
        app.post_login(&login_body).await;

        let new_password = Uuid::new_v4().to_string();
        let pass_body = serde_json::json!({"current_password": app.test_profile.password.as_ref(),
                                                    "new_password": &new_password,
                                                    "new_password_check": &new_password });

        let response = app.post_change_password(&pass_body).await;

        let response: StdResponse = response.json().await.unwrap();
        assert!(response.message.contains("Password Change Successful"));

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn new_password_fields_must_match() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(),
                                                    "password": app.test_profile.password.as_ref()});
        app.post_login(&login_body).await;

        let new_password = Uuid::new_v4().to_string();
        let another_new_password = Uuid::new_v4().to_string();
        let pass_body = serde_json::json!({"current_password": app.test_profile.password.as_ref(),
                                                    "new_password": &new_password, 
                                                    "new_password_check": &another_new_password });

        let response = app.post_change_password(&pass_body).await;

        let response: StdResponse = response.json().await.unwrap();
        assert!(response.message.contains("New Passwords don't Match"));

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn current_password_must_be_valid() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(), 
                                                    "password": app.test_profile.password.as_ref()});
        app.post_login(&login_body).await;

        let new_password = Uuid::new_v4().to_string();
        let wrong_password = Uuid::new_v4().to_string();
        let pass_body = serde_json::json!({"current_password": wrong_password , 
                                                    "new_password": &new_password, 
                                                    "new_password_check": &new_password });

        let response = app.post_change_password(&pass_body).await;

        let response: StdResponse = response.json().await.unwrap();
        assert!(response.message.contains("Current Password is Incorrect!"));

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn changing_password_works() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        // Current Login
        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(), 
                                                    "password": app.test_profile.password.as_ref()});
        app.post_login(&login_body).await;

        // Change Password
        let new_password = Uuid::new_v4().to_string();
        let pass_body = serde_json::json!({"current_password": app.test_profile.password.as_ref(),
                                                    "new_password": &new_password,
                                                    "new_password_check": &new_password });

        let response = app.post_change_password(&pass_body).await;
        let response: StdResponse = response.json().await.unwrap();
        assert!(response.message.contains("Password Change Successful"));

        // Logout
        let response = app.post_logout().await;
        let response: StdResponse = response.json().await.unwrap();
        assert!(
            response
                .message
                .contains("You have successfully logged out.")
        );

        // Login Using New Password
        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(), 
                                                    "password": &new_password});

        let response = app.post_login(&login_body).await;
        let response: StdResponse = response.json().await.unwrap();
        assert!(response.message.contains("Login Successful"));

        // Cleanup DB
        app.drop_test_db().await;
    }
}
