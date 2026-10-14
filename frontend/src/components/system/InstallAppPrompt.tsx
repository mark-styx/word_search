"use client";

import { useEffect, useMemo, useState } from "react";
import { Download, Smartphone } from "lucide-react";
import { Button } from "@/components/ui";

type BeforeInstallPromptEvent = Event & {
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed"; platform: string }>;
};

type IOSNavigator = Navigator & {
  standalone?: boolean;
};

function isIosDevice() {
  if (typeof window === "undefined") {
    return false;
  }

  return /iphone|ipad|ipod/i.test(window.navigator.userAgent);
}

function isStandaloneDisplay() {
  if (typeof window === "undefined") {
    return false;
  }

  const iosNavigator = window.navigator as IOSNavigator;
  return window.matchMedia("(display-mode: standalone)").matches || iosNavigator.standalone === true;
}

export default function InstallAppPrompt() {
  const [mounted, setMounted] = useState(false);
  const [promptEvent, setPromptEvent] = useState<BeforeInstallPromptEvent | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const [standalone, setStandalone] = useState(false);
  const [iosDevice, setIosDevice] = useState(false);

  useEffect(() => {
    setMounted(true);
    setStandalone(isStandaloneDisplay());
    setIosDevice(isIosDevice());

    function handleBeforeInstallPrompt(event: Event) {
      event.preventDefault();
      setPromptEvent(event as BeforeInstallPromptEvent);
    }

    function handleInstalled() {
      setStandalone(true);
      setPromptEvent(null);
    }

    window.addEventListener("beforeinstallprompt", handleBeforeInstallPrompt);
    window.addEventListener("appinstalled", handleInstalled);

    return () => {
      window.removeEventListener("beforeinstallprompt", handleBeforeInstallPrompt);
      window.removeEventListener("appinstalled", handleInstalled);
    };
  }, []);

  const copy = useMemo(() => {
    if (promptEvent) {
      return {
        title: "Install On Your Phone",
        body: "Save WordSearch to your home screen for a full-screen, app-like experience.",
      };
    }

    if (iosDevice) {
      return {
        title: "Add To Home Screen",
        body: "On iPhone or iPad, tap Share in Safari and choose Add to Home Screen.",
      };
    }

    return null;
  }, [iosDevice, promptEvent]);

  if (!mounted || standalone || dismissed || !copy) {
    return null;
  }

  async function handleInstall() {
    if (!promptEvent) {
      return;
    }

    await promptEvent.prompt();
    const choice = await promptEvent.userChoice;
    if (choice.outcome !== "accepted") {
      setDismissed(true);
    }
    setPromptEvent(null);
  }

  return (
    <div className="w-full max-w-xl rounded-3xl border border-primary/20 bg-primary/8 p-4 shadow-sm">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-start gap-3">
          <div className="rounded-2xl bg-primary/12 p-2 text-primary">
            {promptEvent ? <Download size={18} /> : <Smartphone size={18} />}
          </div>
          <div>
            <p className="font-semibold text-text">{copy.title}</p>
            <p className="text-sm text-text-secondary">{copy.body}</p>
          </div>
        </div>
        <div className="flex gap-2">
          {promptEvent ? (
            <Button size="sm" onClick={handleInstall}>
              Install
            </Button>
          ) : null}
          <Button size="sm" variant="ghost" onClick={() => setDismissed(true)}>
            Dismiss
          </Button>
        </div>
      </div>
    </div>
  );
}
