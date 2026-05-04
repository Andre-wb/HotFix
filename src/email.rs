/// Building letter template for verifying the email

use lettre::{
    message::header::ContentType,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters}
    },
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use crate::config::Config;

pub async fn send_verification_code(to_email: &str, code: &str) -> Result<(), String> {
    let config = Config::global();
    let email = Message::builder()
        .from(config.smtp_from
                .parse()
                .map_err(|error| format!("Invalid from address: {error}"))?,
        )
        .to(
            to_email
                .parse()
                .map_err(|error| format!("Invalid to address: {error}"))?,
        )
        .subject("Verification code for HotFix")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("Your verification code is {code}\n\n It expires in 15 minutes. Do not share it with anyone!"))
        .map_err(|error| format!("Failed to build email: {error}"))?;

    let tls_parameters = TlsParameters::new(config.smtp_host.to_string())
        .map_err(|error| format!("Failed to create TLS parameters: {error}"))?;
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(config.smtp_host)
        .port(config.smtp_port)
        .tls(Tls::Wrapper(tls_parameters))
        .credentials(Credentials::new(
            config.smtp_username.to_string(),
            config.smtp_password.to_string(),
        ))
        .build();

    mailer.send(email).await.map_err(|error| format!("Failed to send email: {error}"))?;

    Ok(())
}