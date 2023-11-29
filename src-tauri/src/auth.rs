use anyhow::{ensure, Result};
use oauth2::{
    basic::{BasicClient, BasicTokenType},
    reqwest::async_http_client,
    url::Url,
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields, PkceCodeChallenge,
    PkceCodeVerifier, StandardTokenResponse,
};

use crate::config::ClientConfig;

#[derive(Debug)]
pub struct OAuth2Authorizer {
    authorize_url: Url,
    csrf_state: CsrfToken,
    pkce_verifier: PkceCodeVerifier,
    client: BasicClient,
}

impl OAuth2Authorizer {
    pub fn new(config: ClientConfig) -> OAuth2Authorizer {
        let client = BasicClient::new(
            ClientId::new(config.client_id),
            config.client_secret.map(ClientSecret::new),
            config.preset.auth_url,
            config.preset.token_url,
        );

        let (code_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(config.scopes)
            .set_pkce_challenge(code_challenge)
            .url();
        OAuth2Authorizer {
            authorize_url,
            csrf_state,
            pkce_verifier,
            client,
        }
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        state: &str,
    ) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> {
        let code = AuthorizationCode::new(code.to_string());
        let state = CsrfToken::new(state.to_string());
        ensure!(state.secret() == self.csrf_state.secret());
        let token = self
            .client
            .exchange_code(code)
            .set_pkce_verifier(PkceCodeVerifier::new(
                self.pkce_verifier.secret().to_string(),
            ))
            .request_async(async_http_client)
            .await?;
        Ok(token)
    }

    pub fn authorize_url(&self) -> &Url {
        &self.authorize_url
    }
}
