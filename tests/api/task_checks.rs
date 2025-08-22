use crate::common;

mod tests {
    use std::collections::HashMap;
    use uuid::Uuid;
    use wiremock::{
        Mock, ResponseTemplate,
        matchers::{any, method, path},
    };

    use crate::common::{ConfirmationLinks, TestApp};

    use super::common::spawn_app;

    async fn create_unconfirmed_profile(app: &TestApp) -> ConfirmationLinks {
        let mut body = HashMap::new();
        body.insert("first_name", "Bebeto");
        body.insert("last_name", "Nitro");
        body.insert("email", "n@gmail.com");

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

    async fn create_confirmed_profile(app: &TestApp) {
        let confirmation_link = create_unconfirmed_profile(app).await;
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
        create_unconfirmed_profile(&app).await;

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&app.email_server)
            .await;

        // Act

        // task payload structure
        let task_request_body = serde_json::json!({"profile_id": Uuid::new_v4().to_string(), "task_type": "feature", "source_file": "init.txt"});

        let response = app.post_tasks(task_request_body).await;

        // Assert
        assert_eq!(response.status().as_u16(), 200);

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn tasks_delivered_to_confirmed_profiles() {
        // Arrange
        let mut app = spawn_app().await;
        create_confirmed_profile(&app).await;

        Mock::given(path("v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&app.email_server)
            .await;

        // task payload structure
        let task_request_body = serde_json::json!({"profile_id": Uuid::new_v4().to_string(), "task_type": "feature", "source_file": "init.txt"});

        let response = app.post_tasks(task_request_body).await;

        // Assert
        assert_eq!(response.status().as_u16(), 200);

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn tasks_returns_400_for_invalid_data() {
        let mut app = spawn_app().await;

        let test_cases = vec![
            (
                serde_json::json!({"task_type": "feature", "source_file": "init.txt"}),
                "missing profile_id",
            ),
            (
                serde_json::json!({"profile_id": Uuid::new_v4().to_string()}),
                "missing task_type and source_file",
            ),
        ];

        for (invalid_body, error_message) in test_cases {
            let response = app.post_tasks(invalid_body).await;

            assert_eq!(
                400,
                response.status().as_u16(),
                "The API did not fail with 400 Bad request when the payload was: {}",
                error_message
            );
        }

        app.drop_test_db().await;
    }
}
