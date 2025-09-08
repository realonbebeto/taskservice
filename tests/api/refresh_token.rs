use crate::common;

mod tests {

    use uuid::Uuid;

    use super::common::spawn_app;

    #[actix_web::test]
    async fn valid_refresh_returns_new_access_token() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        // Login and fetch the access token and refresh token
        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(), "password": app.test_profile.password.as_ref()});

        let res1 = app.post_login(&login_body).await;
        // Old  access and refresh tokens
        let access_token1 = res1
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap();

        let cookie1 = res1
            .cookies()
            .find(|c| c.name() == "refresh_token")
            .map(|c| c)
            .unwrap();

        let refresh_token1 = cookie1.value();

        assert_eq!(res1.status().as_u16(), 200);

        // using time to cause different access and refresh tokens
        // without this generated access and refresh tokens (old and new) are same
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Hit refresh endpoint
        let res2 = app.refresh_token().await;

        dbg!(&res2);

        let access_token2 = res2
            .headers()
            .get("Authorization")
            .unwrap()
            .to_str()
            .unwrap();

        let cookie2 = res2
            .cookies()
            .find(|c| c.name() == "refresh_token")
            .map(|c| c)
            .unwrap();

        let refresh_token2 = cookie2.value();

        // Assert old and new access and refresh tokens
        assert_ne!(access_token1, access_token2);
        assert_ne!(refresh_token1, refresh_token2);

        app.drop_test_db().await;
    }

    fn replace_cookie_value(cookie_header: &str, cookie_name: &str, new_value: &str) -> String {
        let cookies: Vec<&str> = cookie_header.split(";").collect();
        let updated_cookies = cookies
            .iter()
            .map(|c| {
                let trimmed_cookie = c.trim();
                if trimmed_cookie.starts_with(&format!("{}=", cookie_name)) {
                    format!("{}={}", cookie_name, new_value)
                } else {
                    trimmed_cookie.to_string()
                }
            })
            .collect::<Vec<String>>();

        updated_cookies.join("; ")
    }

    #[actix_web::test]
    async fn invalid_or_missing_refresh_token_is_rejected() {
        // Arrange
        let mut app = spawn_app().await;
        app.test_profile.store_test_profile(&app.pool).await;

        // Login and fetch the access token and refresh token
        let login_body = serde_json::json!({"username": app.test_profile.username.as_ref(), "password": app.test_profile.password.as_ref()});

        let r = app.post_login(&login_body).await;
        let cookies = r
            .cookies()
            .map(|c| format!("{}={}", c.name(), c.value()))
            .collect::<Vec<String>>();
        let cookies = cookies.join("; ");
        let new_cookies =
            replace_cookie_value(&cookies, "refresh_token", &Uuid::new_v4().to_string());

        // Hit refresh endpoint with invalid refresh token
        let r2 = app
            .api_client
            .get(format!("{}/admin/refresh-token", &app.address))
            .header("Cookie", new_cookies)
            .send()
            .await
            .unwrap();

        assert_eq!(r2.status().as_u16(), 401);

        let new_cookies = replace_cookie_value(&cookies, "refresh_token", "");

        // Hit refresh endpoint with empty refresh token
        let r3 = app
            .api_client
            .get(format!("{}/admin/refresh-token", &app.address))
            .header("Cookie", new_cookies)
            .send()
            .await
            .unwrap();

        assert_eq!(r3.status().as_u16(), 401);

        app.drop_test_db().await;
    }

    #[actix_web::test]
    async fn missing_refresh_token_is_rejected() {
        // Arrange
        let mut app = spawn_app().await;

        // Login and fetch the access token and refresh token

        // Hit refresh endpoint without refresh token

        app.drop_test_db().await;
    }
}
