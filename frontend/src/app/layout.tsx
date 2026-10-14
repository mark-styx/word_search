import type { Metadata, Viewport } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import Script from "next/script";
import { ThemeProvider } from "@/lib/themes-provider";
import { Header } from "@/components/layout";
import ServiceWorkerRegistration from "@/components/system/ServiceWorkerRegistration";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "WordSearch LAN Party",
  description: "Real-time multiplayer word search game with LAN auto-discovery",
  applicationName: "WordSearch LAN Party",
  manifest: "/manifest.webmanifest",
  appleWebApp: {
    capable: true,
    statusBarStyle: "default",
    title: "WordSearch",
  },
  formatDetection: {
    telephone: false,
  },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  viewportFit: "cover",
  themeColor: "#3b82f6",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="min-h-full flex flex-col bg-bg text-text">
        <Script
          id="browser-extension-shims"
          strategy="beforeInteractive"
          dangerouslySetInnerHTML={{
            __html: `
              window.ethereum = window.ethereum || {};
              window.__firefox__ = window.__firefox__ || {};
              window.__firefox__.reader = window.__firefox__.reader || {};

              (function () {
                var blockedPatterns = [
                  "window.ethereum.selectedAddress",
                  "window.ethereum.emit",
                  "window.__firefox__.reader",
                  "__firefox__"
                ];

                function shouldSuppress(message) {
                  if (!message) return false;
                  return blockedPatterns.some(function (pattern) {
                    return String(message).indexOf(pattern) !== -1;
                  });
                }

                window.addEventListener("error", function (event) {
                  var message = event && (event.message || (event.error && event.error.message));
                  if (!shouldSuppress(message)) return;
                  event.preventDefault();
                  event.stopImmediatePropagation();
                }, true);

                window.addEventListener("unhandledrejection", function (event) {
                  var reason = event && event.reason;
                  var message = reason && (reason.message || reason);
                  if (!shouldSuppress(message)) return;
                  event.preventDefault();
                  event.stopImmediatePropagation();
                }, true);
              })();
            `,
          }}
        />
        <ThemeProvider>
          <ServiceWorkerRegistration />
          <Header />
          {children}
        </ThemeProvider>
      </body>
    </html>
  );
}
