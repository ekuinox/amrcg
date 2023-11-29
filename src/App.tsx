import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

interface ExchangeTokenData {
  name: string;
  accessToken: string;
  refreshToken: string | null;
}

function App() {
  const [name, setName] = useState("");
  const [authUrl, setAuthUrl] = useState<string>("");
  const [tokenResponse, setTokenResponse] = useState<ExchangeTokenData | null>(
    null
  );

  useEffect(() => {
    listen<ExchangeTokenData>("token-response", (r) => {
      if (r.payload.name === name) {
        setTokenResponse(r.payload);
        setAuthUrl("");
        invoke("stop_server", { name });
      }
    });
  }, [name]);

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
            const authUrl = await invoke<string>("start_server", {
              name,
            });
            setAuthUrl(authUrl);
          }}
        >
          generate auth-url
        </button>
        {authUrl.length > 0 && (
          <a href={authUrl} target="_blank">
            open browser
          </a>
        )}
      </div>
      {tokenResponse != null && (
        <div>
          <dl>
            <dt>name</dt>
            <dd>{tokenResponse.name}</dd>
            <dt>accessToken</dt>
            <dd>{tokenResponse.accessToken}</dd>
            <dt>refreshToken</dt>
            <dd>{tokenResponse.refreshToken ?? "null"}</dd>
          </dl>
        </div>
      )}
    </div>
  );
}

export default App;
