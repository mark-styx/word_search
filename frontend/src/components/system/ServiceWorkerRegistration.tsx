"use client";

import { useEffect } from "react";

export default function ServiceWorkerRegistration() {
  useEffect(() => {
    if (process.env.NODE_ENV !== "production") {
      return;
    }

    if (typeof window === "undefined" || !("serviceWorker" in navigator)) {
      return;
    }

    let cancelled = false;

    async function registerServiceWorker() {
      try {
        const registration = await navigator.serviceWorker.register("/sw.js", {
          scope: "/",
        });

        if (!cancelled) {
          void registration.update().catch(() => {});
        }
      } catch (error) {
        console.warn("Service worker registration failed", error);
      }
    }

    if (document.readyState === "complete") {
      void registerServiceWorker();
      return () => {
        cancelled = true;
      };
    }

    const onLoad = () => {
      void registerServiceWorker();
    };
    window.addEventListener("load", onLoad, { once: true });

    return () => {
      cancelled = true;
      window.removeEventListener("load", onLoad);
    };
  }, []);

  return null;
}
