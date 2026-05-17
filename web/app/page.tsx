import Link from "next/link";

export default function Home() {
  return (
    <main
      style={{
        padding: 24,
        fontFamily: "ui-sans-serif, system-ui, sans-serif",
        maxWidth: 640,
      }}
    >
      <h1>Next.js + Clerk + Rust/Axum Auth Lab</h1>
      <p>
        Sign in with Clerk, then visit the protected page. The server component
        calls a Rust/Axum API that verifies your Clerk JWT via JWKS.
      </p>
      <ul style={{ marginTop: 16, lineHeight: 1.8 }}>
        <li>
          <Link href="/sign-in">Sign in</Link>
        </li>
        <li>
          <Link href="/sign-up">Sign up</Link>
        </li>
        <li>
          <Link href="/protected">Go to /protected</Link>
        </li>
      </ul>
    </main>
  );
}
