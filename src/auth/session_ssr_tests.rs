//! session_ssr tests

    use super::{issue_access_token, user_id_from_jwt, JWT_AUDIENCE};
    use chrono::Utc;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde::Serialize;
    use uuid::Uuid;

    const SECRET: &str = "test-secret-at-least-32-bytes-long-aaaa";
    const ISSUER: &str = "https://proj.supabase.co/auth/v1";

    #[derive(Serialize)]
    struct TestClaims {
        sub: String,
        exp: i64,
        iss: String,
        aud: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        nbf: Option<i64>,
    }

    fn encode_token<T: Serialize>(claims: &T) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap()
    }

    fn claims(aud: &str, exp_offset: i64, nbf: Option<i64>) -> TestClaims {
        TestClaims {
            sub: Uuid::new_v4().to_string(),
            exp: Utc::now().timestamp() + exp_offset,
            iss: ISSUER.into(),
            aud: aud.into(),
            nbf,
        }
    }

    #[test]
    fn self_minted_token_roundtrips() {
        let uid = Uuid::new_v4();
        let token = issue_access_token(uid, SECRET, 3600, ISSUER).unwrap();
        assert_eq!(user_id_from_jwt(&token, SECRET, ISSUER).unwrap(), uid);
    }

    #[test]
    fn supabase_style_token_without_nbf_validates() {
        // Supabase access tokens omit `nbf`; validation must still accept them.
        let c = claims(JWT_AUDIENCE, 3600, None);
        let token = encode_token(&c);
        assert_eq!(
            user_id_from_jwt(&token, SECRET, ISSUER)
                .unwrap()
                .to_string(),
            c.sub
        );
    }

    #[test]
    fn wrong_issuer_rejected() {
        let token = issue_access_token(Uuid::new_v4(), SECRET, 3600, ISSUER).unwrap();
        assert!(user_id_from_jwt(&token, SECRET, "https://evil.example/auth/v1").is_err());
    }

    #[test]
    fn wrong_secret_rejected() {
        let token = issue_access_token(Uuid::new_v4(), SECRET, 3600, ISSUER).unwrap();
        assert!(user_id_from_jwt(&token, "a-totally-different-secret-value-zz", ISSUER).is_err());
    }

    #[test]
    fn wrong_audience_rejected() {
        let token = encode_token(&claims("anon", 3600, None));
        assert!(user_id_from_jwt(&token, SECRET, ISSUER).is_err());
    }

    #[test]
    fn missing_issuer_claim_rejected() {
        #[derive(Serialize)]
        struct NoIss {
            sub: String,
            exp: i64,
            aud: String,
        }
        let token = encode_token(&NoIss {
            sub: Uuid::new_v4().to_string(),
            exp: Utc::now().timestamp() + 3600,
            aud: JWT_AUDIENCE.into(),
        });
        assert!(user_id_from_jwt(&token, SECRET, ISSUER).is_err());
    }

    #[test]
    fn expired_token_rejected() {
        let token = encode_token(&claims(JWT_AUDIENCE, -10, None));
        assert!(user_id_from_jwt(&token, SECRET, ISSUER).is_err());
    }

    #[test]
    fn future_nbf_rejected() {
        let token = encode_token(&claims(
            JWT_AUDIENCE,
            3600,
            Some(Utc::now().timestamp() + 600),
        ));
        assert!(user_id_from_jwt(&token, SECRET, ISSUER).is_err());
    }

    #[test]
    fn profile_setup_error_codes_are_safe() {
        use super::ProfileSetupError;

        let dup = ProfileSetupError::SupabaseCreate(
            "User already been registered with this email".into(),
        );
        assert_eq!(dup.auth_query_code(), "github_profile_exists");

        let setup = ProfileSetupError::SupabaseCreate("service unavailable".into());
        assert_eq!(setup.auth_query_code(), "github_profile_setup");

        let db = ProfileSetupError::Database(sqlx::Error::RowNotFound);
        assert_eq!(db.auth_query_code(), "github_profile");
    }
