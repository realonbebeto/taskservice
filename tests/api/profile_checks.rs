use crate::common;

mod tests {

    use super::common::spawn_app;
    use sqlx::Row;
    use std::collections::HashMap;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    #[actix_web::test]
    async fn create_profile_returns_200_for_valid_data() {
        //Arrange
        let mut app = spawn_app().await;

        // Act
        let mut body = HashMap::new();
        body.insert("first_name", "Bebeto");
        body.insert("last_name", "Nitro");
        body.insert("email", "n@gmail.com");

        // Mock server
        Mock::given(path("v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&app.email_server)
            .await;

        // Act
        let response = app.post_profiles(&body).await;

        //Assert
        assert_eq!(200, response.status().as_u16());

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn profile_persists_the_new_profile() {
        //Arrange
        let mut app = spawn_app().await;

        let mut body = HashMap::new();
        body.insert("first_name", "Bebeto");
        body.insert("last_name", "Nitro");
        body.insert("email", "n@gmail.com");

        // Mock server
        Mock::given(path("v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&app.email_server)
            .await;

        app.post_profiles(&body).await;

        let saved = sqlx::query("SELECT email, first_name, status FROM profile")
            .fetch_one(&app.pool)
            .await
            .expect("Failed to fetch profile");

        assert_eq!(saved.get::<String, _>("email"), "n@gmail.com");
        assert_eq!(saved.get::<String, _>("first_name"), "Bebeto");
        assert_eq!(saved.get::<String, _>("status"), "pending_confirmation");

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn profile_returns_400_when_fields_are_present_but_invalid() {
        //Arrange
        let mut app = spawn_app().await;

        let test_cases = vec![
            (
                "first_name= ",
                "last_name=Nitro",
                "email=123@gmail.com",
                "missing first name",
            ),
            (
                "first_name=Bebeto",
                "last_name= ",
                "email=1234@gmail.com",
                "missing last name",
            ),
            (
                "first_name=Bebeto",
                "last_name=Hello",
                "email=definitely-not-an-email",
                "invalid email",
            ),
            (
                "first_name= ",
                "last_name= ",
                "email= ",
                "missing both names and email",
            ),
        ];

        for (fname, lname, email, val_msg) in test_cases {
            // Act
            let mut invalid_body = HashMap::new();
            let fname = fname.split("=").collect::<Vec<&str>>();
            let lname = lname.split("=").collect::<Vec<&str>>();
            let email = email.split("=").collect::<Vec<&str>>();
            invalid_body.insert(fname[0], fname[1]);
            invalid_body.insert(lname[0], lname[1]);
            invalid_body.insert(email[0], email[1]);

            let response = app.post_profiles(&invalid_body).await;

            // Assert
            assert_eq!(
                400,
                response.status().as_u16(), // Addittional customised error message on test failure
                "The API did not return a 200 OK when the payload was {}.",
                val_msg
            )
        }

        app.drop_test_db().await;
    }
    #[actix_web::test]
    async fn create_profile_returns_400_for_missing_data() {
        //Arrange
        let mut app = spawn_app().await;
        let test_cases = vec![
            ("last_name", ("Nitro", "missing first name")),
            ("first_name", ("Bebeto", "missing last name")),
            ("", ("", "missing both first and last names")),
        ];

        for (key, val_message) in test_cases {
            // Act
            let mut invalid_body = HashMap::new();
            invalid_body.insert(key, val_message.0);

            let response = app.post_profiles(&invalid_body).await;

            // Assert
            assert_eq!(
                400,
                response.status().as_u16(), // Addittional customised error message on test failure
                "The API did not fail with 400 Bad Request when the payload was {}.",
                val_message.1
            )
        }

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn profile_sends_confirmation_email_for_valid_data() {
        // Arrange
        let mut app = spawn_app().await;
        let mut body = HashMap::new();
        body.insert("first_name", "Bebeto");
        body.insert("last_name", "Nitro");
        body.insert("email", "x12345@gmail.com");

        Mock::given(path("v3/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&app.email_server)
            .await;

        // Act
        app.post_profiles(&body).await;

        app.drop_test_db().await;
    }
}
