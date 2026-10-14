"use client";

import { Suspense } from "react";
import ProfilePicker from "@/components/auth/ProfilePicker";

export default function LoginPage() {
  return (
    <Suspense fallback={null}>
      <ProfilePicker />
    </Suspense>
  );
}
