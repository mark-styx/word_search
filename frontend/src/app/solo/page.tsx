"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Loader2 } from "lucide-react";
import { PageContainer } from "@/components/layout";
import { Button } from "@/components/ui";
import { api, getAccessToken } from "@/lib/api";

export default function SoloPage() {
  const router = useRouter();
  const [error, setError] = useState("");

  useEffect(() => {
    if (!getAccessToken()) {
      router.push("/login");
      return;
    }
    api
      .soloGame()
      .then(({ id }) => router.replace(`/game/${id}`))
      .catch((err) => setError(err instanceof Error ? err.message : "Failed to start game"));
  }, [router]);

  if (error) {
    return (
      <PageContainer className="flex flex-col items-center justify-center gap-4">
        <p className="text-error">{error}</p>
        <Button onClick={() => router.push("/lobby")}>Back to Lobby</Button>
      </PageContainer>
    );
  }

  return (
    <PageContainer className="flex items-center justify-center">
      <div className="flex items-center gap-2 text-text-secondary">
        <Loader2 className="animate-spin" size={20} /> Starting solo game...
      </div>
    </PageContainer>
  );
}
