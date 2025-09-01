use crate::domain::email::ProfileEmail;
use reqwest::{Client, Url};
use serde::Serialize;

#[derive(Serialize)]
struct Recipient<'a> {
    email: &'a str,
}

#[derive(Serialize)]
struct SendEmailRequest<'a> {
    #[serde(rename = "FromEmail")]
    fromemail: &'a str,
    #[serde(rename = "FromName")]
    fromname: &'a str,
    #[serde(rename = "Subject")]
    subject: &'a str,
    #[serde(rename = "Text-part")]
    text_part: &'a str,
    #[serde(rename = "Html-part")]
    html_part: &'a str,
    #[serde(rename = "Recipients")]
    recipients: Vec<Recipient<'a>>,
}

#[derive(Clone, Debug)]
pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: ProfileEmail,
    private_email_key: String,
    public_email_key: String,
}

impl EmailClient {
    pub fn new(
        base_uri: &str,
        sender: ProfileEmail,
        private_email_key: &str,
        public_email_key: &str,
        timeout: std::time::Duration,
    ) -> Self {
        let base_url = Url::parse(base_uri).expect("Invalid email base uri");
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            base_url,
            sender,
            private_email_key: private_email_key.to_string(),
            public_email_key: public_email_key.to_string(),
        }
    }
    pub async fn send_email(
        &self,
        recipient: &ProfileEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let client_uri = self.base_url.join("v3/send").expect("Invalid email path");

        let request_body = SendEmailRequest {
            fromemail: self.sender.as_ref(),
            recipients: vec![Recipient {
                email: recipient.as_ref(),
            }],
            // TODO! -update correct name
            fromname: "mimi",
            subject,
            html_part: html_content,
            text_part: text_content,
        };

        self.http_client
            .post(client_uri)
            .basic_auth(
                self.public_email_key.clone(),
                Some(self.private_email_key.clone()),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::email::ProfileEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use wiremock::Request;
    use wiremock::matchers::{any, basic_auth, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Generate a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    /// Generate a random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Generate a random email
    fn email() -> ProfileEmail {
        ProfileEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> (EmailClient, String, String) {
        let prek = Faker.fake::<String>();
        let puek = Faker.fake::<String>();
        (
            EmailClient::new(
                &base_url,
                email(),
                &prek,
                &puek,
                std::time::Duration::from_millis(200),
            ),
            puek,
            prek,
        )
    }

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                body.get("FromName").is_some()
                    && body.get("Recipients").is_some()
                    && body.get("Subject").is_some()
                    && body.get("Html-part").is_some()
                    && body.get("Text-part").is_some()
            } else {
                false
            }
        }
    }

    #[actix_web::test]
    async fn send_email_sends_the_expected_request() {
        // Arrange
        let mock_server = MockServer::start().await;
        let (email_client, puek, prek) = email_client(mock_server.uri());

        Mock::given(basic_auth(puek, prek))
            .and(header("Content-Type", "application/json"))
            .and(path("v3/send"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        // Mock expectations are checked on drop
    }

    #[actix_web::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let (email_client, _, _) = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_ok!(outcome);
    }

    #[actix_web::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let (email_client, _, _) = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    #[actix_web::test]
    async fn send_email_timesout_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let (email_client, _, _) = email_client(mock_server.uri());

        // Delay by 3 minutes
        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }
}
