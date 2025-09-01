use crate::common;

mod tests {
    use fake::Fake;
    use fake::faker::internet::en::{Password, Username};
    use std::collections::HashMap;
    use std::time::Duration;
    use uuid::Uuid;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{any, method, path},
    };

    use super::common::spawn_app;
    use crate::common::{ConfirmationLinks, StdResponse, TestApp};
    use crate::test_profile::TestProfile;

    async fn create_unconfirmed_profile(app: &TestApp, profile: &TestProfile) -> ConfirmationLinks {
        // Act
        let mut body = HashMap::new();
        body.insert("first_name", profile.first_name.as_ref());
        body.insert("last_name", profile.first_name.as_ref());
        body.insert("email", profile.email.as_ref());
        body.insert("username", profile.username.as_ref());
        body.insert("password", profile.password.as_ref());

        let _mock_guard = Mock::given(path("/v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .named("Create unconfirmed profile")
            .expect(1)
            .mount_as_scoped(&app.email_server)
            .await;

        app.post_profiles(&body).await.error_for_status().unwrap();

        let email_request = &app
            .email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap();

        app.get_confirmation_links(&email_request)
    }

    async fn create_confirmed_profile(app: &TestApp, profile: &TestProfile) {
        let confirmation_link = create_unconfirmed_profile(app, profile).await;
        reqwest::get(confirmation_link.html)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    #[actix_web::test]
    async fn tasks_not_delivered_to_unconfirmed_profiles() {
        // Arrange
        let mut app = spawn_app().await;
        create_unconfirmed_profile(&app, &app.test_profile).await;

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&app.email_server)
            .await;

        // Login
        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(),
                                                    "password": app.test_profile.password.as_ref()});
        app.post_login(&login_body).await;

        // task payload structure
        let task_request_body = serde_json::json!({"task_type": "feature", "source_file": "init.txt", "idempotency_key": Uuid::new_v4().to_string()});

        let response = app.post_tasks(&task_request_body).await;

        // Assert
        assert_eq!(response.status().as_u16(), 200);

        app.dispatch_all_pending_emails().await;

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn tasks_are_delivered_to_confirmed_profiles() {
        // Arrange
        let mut app = spawn_app().await;
        create_confirmed_profile(&app, &app.test_profile).await;

        Mock::given(path("v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&app.email_server)
            .await;

        // Login
        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(),
                                                    "password": app.test_profile.password.as_ref()});
        app.post_login(&login_body).await;

        // task payload structure
        let task_request_body = serde_json::json!({"task_type": "feature", "source_file": "init.txt", "idempotency_key": Uuid::new_v4().to_string()});

        let response = app.post_tasks(&task_request_body).await;

        // Assert
        assert_eq!(response.status().as_u16(), 200);

        app.dispatch_all_pending_emails().await;

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn tasks_returns_400_for_invalid_data() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        // Login
        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(),
                                                    "password": app.test_profile.password.as_ref()});
        app.post_login(&login_body).await;

        let test_cases = vec![
            (
                serde_json::json!({"task_type": "feature"}),
                "missing source_file",
            ),
            (
                serde_json::json!({"profile_id": Uuid::new_v4().to_string()}),
                "missing task_type and source_file",
            ),
        ];

        for (invalid_body, error_message) in test_cases {
            let response = app.post_tasks(&invalid_body).await;

            assert_eq!(
                400,
                response.status().as_u16(),
                "The API did not fail with 400 Bad request when the payload was: {}",
                error_message
            );
        }

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn requests_missing_authorized_session_are_rejected() {
        // Arrange
        let mut app = spawn_app().await;

        // task payload structure
        let task_request_body =
            serde_json::json!({"task_type": "feature", "source_file": "init.txt"});

        let response = app.post_tasks(&task_request_body).await;

        // Assert
        assert_eq!(401, response.status().as_u16());
        assert_eq!(
            r#"Basic realm="task-service""#,
            response.headers()["WWW-Authenticate"]
        );

        // Cleanup
        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn non_existing_profile_is_rejected() {
        // Arrange
        let mut app = spawn_app().await;

        //Random credentials
        let username = Username().fake::<String>();
        let password = Password(std::ops::Range { start: 8, end: 16 }).fake::<String>();

        // Login
        let login_body = serde_json::json!({"username": username,
                                                    "password": password});
        app.post_login(&login_body).await;

        let task_request_body =
            serde_json::json!({"task_type": "feature", "source_file": "init.txt"});

        let response = app.post_tasks(&task_request_body).await;

        //Assert
        assert_eq!(401, response.status().as_u16());
        assert_eq!(
            r#"Basic realm="task-service""#,
            response.headers()["WWW-Authenticate"]
        );

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn invalid_password_is_rejected() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        //Random password
        let password = Password(std::ops::Range { start: 8, end: 16 }).fake::<String>();
        assert_ne!(app.test_profile.password.as_ref(), password);

        // Login
        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(),
                                                    "password": password});
        app.post_login(&login_body).await;

        let task_request_body =
            serde_json::json!({"task_type": "feature", "source_file": "init.txt"});

        let response = app.post_tasks(&task_request_body).await;

        //Assert
        assert_eq!(401, response.status().as_u16());
        assert_eq!(
            r#"Basic realm="task-service""#,
            response.headers()["WWW-Authenticate"]
        );

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn task_creation_is_idempotent() {
        // Arrange
        let mut app = spawn_app().await;
        create_confirmed_profile(&app, &app.test_profile).await;

        // Login
        app.test_profile.post_login(&app).await;

        Mock::given(path("/v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&app.email_server)
            .await;

        let task_request_body = serde_json::json!({"task_type": "feature", "source_file": "init.txt", "idempotency_key": Uuid::new_v4().to_string()});

        // First Request
        let response = app.post_tasks(&task_request_body).await;

        assert_eq!(response.status().as_u16(), 200);
        let r: StdResponse = response.json().await.unwrap();
        assert!(r.message.contains("Task successfully created"));

        // Second Request
        let response = app.post_tasks(&task_request_body).await;

        assert_eq!(response.status().as_u16(), 200);
        let r: StdResponse = response.json().await.unwrap();
        assert!(r.message.contains("Task successfully created"));

        app.dispatch_all_pending_emails().await;

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn concurrent_task_submission_is_handled_gracefully() {
        // Arrange
        let mut app = spawn_app().await;
        create_confirmed_profile(&app, &app.test_profile).await;

        // Login
        app.test_profile.post_login(&app).await;

        Mock::given(path("/v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .expect(1)
            .mount(&app.email_server)
            .await;

        let task_request_body = serde_json::json!({"task_type": "feature", "source_file": "init.txt", "idempotency_key": Uuid::new_v4().to_string()});

        let res1 = app.post_tasks(&task_request_body);
        let res2 = app.post_tasks(&task_request_body);

        let (res1, res2) = tokio::join!(res1, res2);

        assert_eq!(res1.status(), res2.status());
        assert_eq!(res1.text().await.unwrap(), res2.text().await.unwrap());

        app.dispatch_all_pending_emails().await;

        app.drop_test_db().await;
    }
}
