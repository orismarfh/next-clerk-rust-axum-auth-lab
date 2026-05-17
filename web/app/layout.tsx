import type { Metadata } from "next";
import Link from "next/link";
import {
  ClerkProvider,
  Show,
  SignInButton,
  SignUpButton,
  UserButton,
} from "@clerk/nextjs";
import "./globals.css";

export const metadata: Metadata = {
  title: "Next.js + Clerk + Rust/Axum Auth Lab",
  description:
    "Reference monorepo: Next.js (App Router) + Clerk frontend calling a Rust/Axum API that verifies Clerk JWTs against JWKS.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <ClerkProvider>
      <html lang="en">
        <body>
          <header
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "12px 24px",
              borderBottom: "1px solid #e5e5e5",
              fontFamily: "ui-sans-serif, system-ui, sans-serif",
            }}
          >
            <Link
              href="/"
              style={{ fontWeight: 600, textDecoration: "none", color: "inherit" }}
            >
              auth-lab
            </Link>
            <nav style={{ display: "flex", gap: 12, alignItems: "center" }}>
              <Link href="/protected">/protected</Link>
              <Show when="signed-out">
                <SignInButton mode="modal" />
                <SignUpButton mode="modal" />
              </Show>
              <Show when="signed-in">
                <UserButton />
              </Show>
            </nav>
          </header>
          {children}
        </body>
      </html>
    </ClerkProvider>
  );
}
