import { auth } from "@clerk/nextjs/server";

export default async function ProtectedPage() {
  const { getToken } = await auth();
  const token = await getToken();
  if (!token) return <p>Not signed in.</p>;

  const base = process.env.NEXT_PUBLIC_API_BASE_URL!;
  const res = await fetch(`${base}/me`, {
    headers: { Authorization: `Bearer ${token}` },
    cache: "no-store",
  });
  const body = await res.text();

  return (
    <main style={{ padding: 24, fontFamily: "ui-monospace, monospace" }}>
      <h1>/protected</h1>
      <p>Status: {res.status}</p>
      <pre style={{ whiteSpace: "pre-wrap" }}>{body}</pre>
    </main>
  );
}
