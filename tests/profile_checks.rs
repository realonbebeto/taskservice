mod common;

mod tests {

    use super::common::spawn_app;
    use sqlx::Row;
    use std::collections::HashMap;

    #[actix_web::test]
    async fn create_profile_returns_200_for_valid_data() {
        //Arrange
        let app = spawn_app().await;
        let client = reqwest::Client::new();

        // Act
        let mut body = HashMap::new();
        body.insert("first_name", "Bebeto");
        body.insert("last_name", "Nitro");
        body.insert("email", "n@gmail.com");

        let response = client
            .post(&format!("{}/profile", &app.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request");

        //Assert
        assert_eq!(200, response.status().as_u16());

        let saved = sqlx::query("SELECT email, first_name FROM profile")
            .fetch_one(&app.pool)
            .await
            .expect("Failed to fetch profile");

        assert_eq!(saved.get::<String, _>("email"), "n@gmail.com");
        assert_eq!(saved.get::<String, _>("first_name"), "Bebeto");
    }

    #[actix_web::test]
    async fn create_profile_returns_400_for_missing_data() {
        //Arrange
        let app = spawn_app().await;
        let client = reqwest::Client::new();
        let test_cases = vec![
            ("last_name", ("Nitro", "missing first name")),
            ("first_name", ("Bebeto", "missing last name")),
            ("", ("", "missing both first and last names")),
        ];

        for (key, val_message) in test_cases {
            // Act
            let mut invalid_body = HashMap::new();
            invalid_body.insert(key, val_message.0);

            let response = client
                .post(&format!("{}/profile", &app.address))
                .json(&invalid_body)
                .send()
                .await
                .expect("Failed to execute request");

            // Assert
            assert_eq!(
                400,
                response.status().as_u16(), // Addittional customised error message on test failure
                "The API did not fail with 400 Bad Request when the payload was {}.",
                val_message.1
            )
        }
    }
}
