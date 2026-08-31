mod tests {
    use super::{
        build_authorization_url, extract_callback_params, pkce_challenge, validate_callback_state,
        OAuthRequest,
    };

    #[test]
    fn authorization_url_includes_pkce_state_and_loopback_redirect() {
        let url = build_authorization_url(&OAuthRequest {
            client_id: "client-id",
            redirect_uri: "http://127.0.0.1:8123",
            state: "state-123",
            scope: "scope-a scope-b",
            code_challenge: "challenge-123",
        });

        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A8123"));
        assert!(url.contains("state=state-123"));
        assert!(url.contains("code_challenge=challenge-123"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
    }

    #[test]
    fn callback_parser_extracts_code_state_and_error() {
        let callback =
            extract_callback_params("GET /?code=abc123&state=expected&error=denied HTTP/1.1")
                .unwrap();

        assert_eq!(callback.code.as_deref(), Some("abc123"));
        assert_eq!(callback.state.as_deref(), Some("expected"));
        assert_eq!(callback.error.as_deref(), Some("denied"));
    }

    #[test]
    fn callback_state_validation_rejects_mismatches() {
        validate_callback_state("expected", Some("expected")).unwrap();
        let err = validate_callback_state("expected", Some("wrong")).unwrap_err();
        assert!(err.to_string().contains("state mismatch"));
    }

    #[test]
    fn pkce_challenge_is_url_safe() {
        let challenge = pkce_challenge("0123456789abcdef0123456789abcdef0123456789");
        assert!(!challenge.contains('='));
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
    }
}
