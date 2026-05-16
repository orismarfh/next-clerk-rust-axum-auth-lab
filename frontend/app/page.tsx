import AuthLab from "./auth-lab";
import styles from "./page.module.css";

export default function Home() {
  return (
    <main className={styles.page}>
      <h1>next-clerk-rust-axum-auth-lab</h1>
      <p className={styles.subtitle}>
        Starter project and research playground for Next.js + Clerk authentication with a Rust
        Axum backend.
      </p>
      <AuthLab />
    </main>
  );
}
