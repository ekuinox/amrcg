use std::collections::HashMap;

use anyhow::{bail, ensure, Result};
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

    pub async fn try_into_token_with_redirect_url(
        self,
        redirect_url: Url,
    ) -> Result<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> {
        let params = redirect_url.query_pairs().collect::<HashMap<_, _>>();
        let code = match params.get("code") {
            Some(code) => AuthorizationCode::new(code.to_string()),
            None => bail!("couldn't find pair which key is 'code'"),
        };
        let state = match params.get("state") {
            Some(state) => CsrfToken::new(state.to_string()),
            None => bail!("couldn't find pair which key is 'state'"),
        };
        ensure!(state.secret() == self.csrf_state.secret());
        let token = self
            .client
            .exchange_code(code)
            .set_pkce_verifier(self.pkce_verifier)
            .request_async(async_http_client)
            .await?;
        Ok(token)
    }

    pub fn authorize_url(&self) -> &Url {
        &self.authorize_url
    }
}
