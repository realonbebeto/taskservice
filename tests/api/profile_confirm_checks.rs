use crate::common;
mod tests {
    use super::common::spawn_app;
    use sqlx::Row;
    use std::collections::HashMap;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    #[actix_web::test]
    async fn confirmations_without_token_are_rejected_with_a_400() {
        // Arrange
        let mut app = spawn_app().await;

        let req_uri = format!("{}/profile/confirm", app.address);
        dbg!(&req_uri);
        let response = reqwest::get(&req_uri).await.unwrap();

        //Assert
        assert_eq!(response.status().as_u16(), 400);

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn create_profile_sends_a_confirmation_email_with_a_link() {
        // Arrange
        let mut app = spawn_app().await;
        let mut body = HashMap::new();
        body.insert("first_name", "Bebeto");
        body.insert("last_name", "Nitro");
        body.insert("email", "x12345@gmail.com");

        Mock::given(path("v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&app.email_server)
            .await;

        app.post_profiles(&body).await;

        let email_request = &app.email_server.received_requests().await.unwrap()[0];

        let confirmation_links = app.get_confirmation_links(&email_request);

        assert_eq!(confirmation_links.html, confirmation_links.plain_text);

        // Clean Up
        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn link_returned_by_profile_create_returns_200_if_called() {
        // Arrange
        let mut app = spawn_app().await;
        let mut body = HashMap::new();
        body.insert("first_name", "Bebeto");
        body.insert("last_name", "Nitro");
        body.insert("email", "x12345@gmail.com");

        Mock::given(path("/v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&app.email_server)
            .await;

        app.post_profiles(&body).await;

        let email_request = &app.email_server.received_requests().await.unwrap()[0];

        let confirmation_links = app.get_confirmation_links(&email_request);

        let response = reqwest::get(confirmation_links.html).await.unwrap();

        assert_eq!(response.status().as_u16(), 200);

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn clicking_on_the_confirmation_link_confirms_a_profile() {
        // Arrange
        let mut app = spawn_app().await;
        let mut body = HashMap::new();
        body.insert("first_name", "Bebeto");
        body.insert("last_name", "Nitro");
        body.insert("email", "x12345@gmail.com");

        Mock::given(path("/v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&app.email_server)
            .await;

        app.post_profiles(&body).await;

        let email_request = &app.email_server.received_requests().await.unwrap()[0];

        let confirmation_links = app.get_confirmation_links(&email_request);

        reqwest::get(confirmation_links.html).await.unwrap();

        //Assert
        let saved = sqlx::query("SELECT email, first_name, status FROM profile")
            .fetch_one(&app.pool)
            .await
            .expect("Failed to fetch profile");

        assert_eq!(saved.get::<String, _>("email"), "x12345@gmail.com");
        assert_eq!(saved.get::<String, _>("first_name"), "Bebeto");
        assert_eq!(saved.get::<String, _>("status"), "confirmed");

        app.drop_test_db().await;
    }
}
