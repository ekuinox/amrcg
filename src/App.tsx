import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";

function App() {
  const [name, setName] = useState("");
  const [authUrl, setAuthUrl] = useState<string>("");
  const [redirectUrl, setRedirectUrl] = useState<string>("");
  const [tokenResponse, setTokenResponse] = useState<unknown>(null);

  return (
    <div className="container">
      <div>
        <input
          placeholder="name"
          onChange={(e) => setName(e.target.value)}
          value={name}
        />
        <button
          onClick={async () => {
            const authUrl = await invoke<string>("get_authorize_url", {
              name,
            });
            setAuthUrl(authUrl);
          }}
        >
          generate auth-url
        </button>
        {authUrl.length > 0 && (
          <a href={authUrl} target="_blank">
            auth-url
          </a>
        )}
      </div>
      <div>
        <input
          placeholder="redirect-url"
          onChange={(e) => setRedirectUrl(e.target.value)}
          value={redirectUrl}
        />
        <button
          onClick={async () => {
            const tokenResponse = await invoke<unknown>(
              "exchange_redirect_url",
              {
                name,
                redirectUrl,
              }
            );
            setTokenResponse(tokenResponse);
          }}
        >
          exchange redirect-url
        </button>
        <p>{JSON.stringify(tokenResponse)}</p>
      </div>
    </div>
  );
}

export default App;
