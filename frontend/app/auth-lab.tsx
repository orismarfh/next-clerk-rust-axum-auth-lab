"use client";

import {
  SignInButton,
  SignUpButton,
  UserButton,
  useAuth,
} from "@clerk/nextjs";
import { useMemo, useState } from "react";
import styles from "./page.module.css";

type ApiResult = {
  status: number;
  body: unknown;
};

export default function AuthLab() {
  const { getToken, isLoaded, isSignedIn } = useAuth();
  const [token, setToken] = useState<string>("");
  const [publicResult, setPublicResult] = useState<ApiResult | null>(null);
  const [protectedResult, setProtectedResult] = useState<ApiResult | null>(null);
  const [error, setError] = useState<string>("");

  const backendUrl =
    process.env.NEXT_PUBLIC_BACKEND_URL?.replace(/\/$/, "") ??
    "http://localhost:4000";

  const tokenPreview = useMemo(() => {
    if (!token) {
      return "";
    }

    if (token.length <= 20) {
      return token;
    }

    return `${token.slice(0, 12)}...${token.slice(-8)}`;
  }, [token]);

  const callPublicRoute = async () => {
    setError("");
    const response = await fetch(`${backendUrl}/api/public`);
    const body = await response.json();
    setPublicResult({ status: response.status, body });
  };

  const callProtectedRoute = async () => {
    setError("");
    setProtectedResult(null);

    if (!isLoaded || !isSignedIn) {
      setError("Sign in first to request a Clerk JWT.");
      return;
    }

    const clerkJwt = await getToken();

    if (!clerkJwt) {
      setError("Clerk did not return a JWT. Check Clerk setup/environment.");
      return;
    }

    setToken(clerkJwt);

    const response = await fetch(`${backendUrl}/api/protected`, {
      headers: {
        Authorization: `Bearer ${clerkJwt}`,
      },
    });
    const body = await response.json();
    setProtectedResult({ status: response.status, body });
  };

  return (
    <section className={styles.card}>
      <h2>Clerk + Axum Authentication Lab</h2>
      <p>
        Sign in/up with Clerk, request a JWT in the browser via <code>getToken()</code>, and call
        Axum routes using <code>Authorization: Bearer &lt;token&gt;</code>.
      </p>

      <div className={styles.authRow}>
        {!isSignedIn ? (
          <>
          <SignInButton mode="modal">
            <button className={styles.primaryButton} type="button">
              Sign in
            </button>
          </SignInButton>
          <SignUpButton mode="modal">
            <button className={styles.secondaryButton} type="button">
              Sign up
            </button>
          </SignUpButton>
          </>
        ) : (
          <UserButton />
        )}
      </div>

      <div className={styles.actions}>
        <button className={styles.primaryButton} type="button" onClick={callPublicRoute}>
          Call public backend route
        </button>
        <button className={styles.secondaryButton} type="button" onClick={callProtectedRoute}>
          Call protected backend route
        </button>
      </div>

      {error ? <p className={styles.error}>{error}</p> : null}

      {tokenPreview ? (
        <div className={styles.responseBlock}>
          <h3>JWT preview from Clerk</h3>
          <code>{tokenPreview}</code>
        </div>
      ) : null}

      {publicResult ? (
        <div className={styles.responseBlock}>
          <h3>Public route response ({publicResult.status})</h3>
          <pre>{JSON.stringify(publicResult.body, null, 2)}</pre>
        </div>
      ) : null}

      {protectedResult ? (
        <div className={styles.responseBlock}>
          <h3>Protected route response ({protectedResult.status})</h3>
          <pre>{JSON.stringify(protectedResult.body, null, 2)}</pre>
        </div>
      ) : null}
    </section>
  );
}
