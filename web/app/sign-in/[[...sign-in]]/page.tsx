import { SignIn } from "@clerk/nextjs";

export default function Page() {
  return (
    <main
      style={{
        display: "flex",
        justifyContent: "center",
        padding: 48,
      }}
    >
      <SignIn />
    </main>
  );
}
