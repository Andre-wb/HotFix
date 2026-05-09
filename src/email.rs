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

    if config.is_development() {
        tracing::info!("========================================");
        tracing::info!("Verification code for {}", to_email);
        tracing::info!("Code: {}", code);
        tracing::info!("========================================");
        return Ok(());
    }

    tracing::debug!("Attempting to send email to {} via {}:{}", to_email, config.smtp_host, config.smtp_port);

    let email = Message::builder()
        .from(config.smtp_from
                  .parse()
                  .map_err(|error| format!("Invalid from address '{}': {}", config.smtp_from, error))?,
        )
        .to(
            to_email
                .parse()
                .map_err(|error| format!("Invalid to address '{}': {}", to_email, error))?,
        )
        .subject("Verification code for HotFix")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("Hello, it's HotFix!\n Your verification code is: {}\n\nIt expires in 15 minutes. Do not share it with anyone. If you haven't tried to sign in, ignore that message.", code))
        .map_err(|error| format!("Failed to build email: {}", error))?;

    let mailer = match config.smtp_port {
        465 => {
            tracing::debug!("Using TLS wrapper for port {}", config.smtp_host);
            let tls_parameters = TlsParameters::new(config.smtp_host.to_string())
                .map_err(|error| format!("Failed to create TLS parameters: {}", error))?;
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(config.smtp_host)
                .port(config.smtp_port)
                .tls(Tls::Wrapper(tls_parameters))
                .credentials(Credentials::new(
                    config.smtp_username.to_string(),
                    config.smtp_password.to_string(),
                ))
                .build()
        }
        _ => {
            tracing::debug!("Using STARTTLS for port {}", config.smtp_port);
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(config.smtp_host)
                .map_err(|error| format!("Failed to create SMTP relay: {}. Check SMTP_HOST and SMTP_PORT.", error))?
                .port(config.smtp_port)
                .credentials(Credentials::new(
                    config.smtp_username.to_string(),
                    config.smtp_password.to_string(),
                ))
                .build()
        }
    };

    match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        mailer.send(email)
    ).await {
        Ok(Ok(response)) => {
            tracing::info!("Email sent successfully: {:?}", response);
            Ok(())
        }
        Ok(Err(error)) => {
            tracing::error!("SMTP error: {}", error);
            Err(format!("Failed to send email: {}. Check your SMTP_USERNAME, SMTP_PASSWORD, and that your email provider allows SMTP access.", error))
        }
        Err(_) => {
            tracing::error!("Email sending timed out after 15 seconds");
            Err("Email sending timed out. Check your SMTP_HOST and SMTP_PORT settings.".to_string())
        }
    }
}