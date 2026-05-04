#[cfg(test)]
mod tests {
    use hotfix::{
        RegisterForm,
        db::validate_registration,
    };
    use hotfix::db::{hash_password, verify_password};

    #[test]
    fn test_username_too_short() {
        let form = RegisterForm {
            username: "ab".to_string(),
            email: "test@test.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret123".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_username_too_long() {
        let form = RegisterForm {
            username: "a".repeat(31),
            email: "test@test.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret123".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_username_invalid_characters() {
        let form = RegisterForm {
            username: "test-user!".to_string(),
            email: "test@test.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret123".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_username_valid() {
        let form = RegisterForm {
            username: "valid_username123".to_string(),
            email: "test@test.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret123".to_string(),
        };
        assert!(validate_registration(&form).is_ok());
    }

    #[test]
    fn test_email_missing_at_symbol() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "testtest.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret123".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_email_missing_dot() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@testcom".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret123".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_email_valid() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "valid.email@test.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret123".to_string(),
        };
        assert!(validate_registration(&form).is_ok());
    }

    #[test]
    fn test_password_too_short() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@test.com".to_string(),
            password: "Sec1".to_string(),
            confirm_password: "Sec1".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_password_no_uppercase() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@test.com".to_string(),
            password: "secret123".to_string(),
            confirm_password: "secret123".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_password_no_lowercase() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@test.com".to_string(),
            password: "SECRET123".to_string(),
            confirm_password: "SECRET123".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_password_no_number() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@test.com".to_string(),
            password: "SecretSecret".to_string(),
            confirm_password: "SecretSecret".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_password_valid() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@test.com".to_string(),
            password: "ValidPass123".to_string(),
            confirm_password: "ValidPass123".to_string(),
        };
        assert!(validate_registration(&form).is_ok());
    }

    #[test]
    fn test_passwords_do_not_match() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@test.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret1234".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_passwords_match() {
        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@test.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret123".to_string(),
        };
        assert!(validate_registration(&form).is_ok());
    }

    #[test]
    fn test_incorrect_password_validation() {
        let invalid_passwords = vec![
            "secret123".to_string(),
            "Secret1".to_string(),
            "secretsecret".to_string(),
        ];

        for password in invalid_passwords {
            assert!(
                validate_registration(&RegisterForm {
                    username: "test99".to_string(),
                    email: "test@test.com".to_string(),
                    password: password.clone(),
                    confirm_password: password,
                }).is_err()
            );
        }

        let form = RegisterForm {
            username: "test99".to_string(),
            email: "test@test.com".to_string(),
            password: "Secret123".to_string(),
            confirm_password: "Secret1234".to_string(),
        };
        assert!(validate_registration(&form).is_err());
    }

    #[test]
    fn test_successful_registration() {
        let form = RegisterForm {
            username: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password: "SecurePass123".to_string(),
            confirm_password: "SecurePass123".to_string(),
        };
        assert!(validate_registration(&form).is_ok());
    }

    #[test]
    fn test_password_hashing() {
        assert!(hash_password("secret123").is_ok());
    }

    #[test]
    fn test_password_verifying() {
        let password = "secret123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).is_ok());
    }
}